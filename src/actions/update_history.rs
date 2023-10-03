use chrono::{DateTime, Datelike, NaiveTime, TimeZone, Timelike, Utc};
use futures::future::try_join_all;

use crate::{
    esi::{get_market_history, get_market_region_types, EsiClient},
    repository::{ItemRepository, MarketHistoryRepository},
};

use super::UpdateError;

pub async fn update_history_for_region(
    region_id: usize,
    mut market_history_repository: MarketHistoryRepository,
    mut item_repository: ItemRepository,
) -> Result<(), UpdateError> {
    let client = EsiClient::current();

    log::debug!("Starting history for region: {}", region_id);

    let today = current_market_date();

    let latest_histories = market_history_repository
        .latest_histories(region_id)
        .await
        .map_err(|e| UpdateError::MarketHistorySql(e, region_id))?;

    let all_items = item_repository
        .tradeable_item_ids()
        .await
        .map_err(|e| UpdateError::MarketHistorySql(e, region_id))?;

    let region_types = get_market_region_types(client.clone(), region_id)
        .await
        .map_err(|e| UpdateError::MarketHistoryEsi(e, region_id))?
        .into_iter()
        .map(|i| i as usize)
        .filter(|i| all_items.contains(&i)) // needs to be published
        .filter(|i| latest_histories.get(i).map(|s| *s < today).unwrap_or(true))
        .collect::<Vec<_>>();

    log::debug!(
        "Latest histories: {}, amount of types: {}",
        latest_histories.len(),
        region_types.len()
    );

    let chunk_size = 300;
    let chunk_len = region_types.len() / chunk_size;

    for (chunk, types) in region_types.chunks(chunk_size).enumerate() {
        let a = try_join_all(types.iter().map(|type_id| async {
            get_market_history(client.clone(), region_id, *type_id)
                .await
                .map(|history| (*type_id, history))
        }))
        .await;

        match a {
            Ok(added) => {
                let added = added
                    .into_iter()
                    .flat_map(|(id, history)| {
                        history
                            .into_iter()
                            .filter(|item| {
                                if let Some(latest) = latest_histories.get(&id) {
                                    return Utc.from_utc_datetime(
                                        &item
                                            .date
                                            .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
                                    ) > *latest;
                                }
                                true
                            })
                            .map(|item| (id, item))
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();

                let _ = market_history_repository
                    .insert_items(added, region_id)
                    .await
                    .map_err(|e| UpdateError::MarketHistorySql(e, region_id));
                log::info!(
                    "Collected history for region: {}  chunk({}/{})",
                    region_id,
                    chunk,
                    chunk_len
                );
            }
            Err(e) => log::info!(
                "Failed collecting history for region: {}, chunk({}/{}), {:?}",
                region_id,
                chunk,
                chunk_len,
                e
            ),
        }
    }

    Ok(())
}

fn current_market_date() -> DateTime<Utc> {
    let today = Utc::now();
    let today = if today.hour() < 11 {
        Utc.with_ymd_and_hms(today.year(), today.month(), today.day() - 2, 11, 0, 0)
            .unwrap()
    } else {
        Utc.with_ymd_and_hms(today.year(), today.month(), today.day() - 1, 11, 0, 0)
            .unwrap()
    };
    today
}
