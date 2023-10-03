use crate::{
    esi::{create_client, get_market_history, get_market_orders, get_market_region_types},
    repository::{ItemRepository, MarketHistoryRepository},
};
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, TimeZone, Timelike, Utc};
use rocket::{futures::future::try_join_all, tokio::sync::Mutex};
use sqlx::{pool::PoolConnection, Acquire, Sqlite, SqlitePool};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

pub async fn update_history_for_region(region_id: usize, db: Arc<Mutex<SqlitePool>>) {
    let client = create_client();

    println!("Starting history for region: {}", region_id);

    let today = current_market_date();

    let mut market_history_repository = MarketHistoryRepository::new(db.clone());
    let mut item_repository = ItemRepository::new(db.clone());

    let latest_histories = market_history_repository
        .latest_histories(region_id)
        .await
        .expect("Can not get latest histories");

    let all_items = item_repository.tradeable_item_ids().await.unwrap();

    let region_types = get_market_region_types(&client, region_id)
        .await
        .expect("error getting region types")
        .into_iter()
        .map(|i| i as usize)
        .filter(|i| all_items.contains(&i)) // needs to be published
        .filter(|i| latest_histories.get(i).map(|s| *s < today).unwrap_or(true))
        .collect::<Vec<_>>();

    println!(
        "Latest histories: {}, amount of types: {}",
        latest_histories.len(),
        region_types.len()
    );

    let mut added = Vec::with_capacity(region_types.len());

    let chunk_size = 100;
    let chunk_len = region_types.len() / chunk_size;

    for (chunk, types) in region_types.chunks(chunk_size).enumerate() {
        let a = try_join_all(types.iter().map(|type_id| async {
            get_market_history(&client, region_id, *type_id)
                .await
                .map(|history| (*type_id, history))
        }))
        .await;

        match a {
            Ok(x) => {
                added.extend(x);
                println!(
                    "Collected orders for region: {}  chunk({}/{})",
                    region_id, chunk, chunk_len
                );
            }
            Err(e) => println!(
                "Failed collecting orders for region: {}, chunk({}/{}), {:?}",
                region_id, chunk, chunk_len, e
            ),
        }
    }

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

    let mut db = db.lock().await.acquire().await.unwrap();
    if let Err(e) = insert_market_history_items(&mut db, added, region_id).await {
        sqlx::query!("ROLLBACK").execute(&mut db).await.unwrap();
        panic!("Error inserting items: {}", e);
    }
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

async fn insert_market_history_items(
    db: &mut PoolConnection<Sqlite>,
    added: Vec<(usize, crate::esi::models::MarketRegionHistoryItem)>,
    region_id: usize,
) -> Result<(), sqlx::Error> {
    sqlx::query!("BEGIN TRANSACTION")
        .execute(db.as_mut())
        .await
        .unwrap();

    for (item_id, history) in added {
        let item_id = item_id as i64;
        let region_id = region_id as i64;
        sqlx::query!("INSERT INTO market_history (date, item_id, region_id, low_price, high_price, average_price, order_count, volume) VALUES (?, ?, ?, ?, ?, ?, ?, ?)", 
            history.date, item_id, region_id, history.lowest, history.highest, history.average, history.order_count, history.volume
        ).execute(db.as_mut()).await?;
    }
    sqlx::query!("COMMIT").execute(db.as_mut()).await?;

    Ok(())
}

pub async fn update_order_for_region(
    region: usize,
    mut db: PoolConnection<Sqlite>,
) -> Result<(), sqlx::Error> {
    let client = create_client();

    println!("Starting orders for region: {}", region);

    let orders = get_market_orders(&client, region).await;

    let orders = match orders {
        Ok(orders) => orders,
        Err(e) => {
            panic!("Error getting region orders: {:?}", e);
        }
    };

    println!("Region: {}, orders: {}", region, orders.len());

    let mut transaction = db.acquire().await.unwrap().begin().await.unwrap();

    for order in orders {
        let expiry = order.issued + chrono::Duration::days(order.duration as i64);
        let order_id = order.order_id as i64;
        let type_id = order.type_id as i64;
        let system_id = order.system_id as i64;
        let volume_remain = order.volume_remain as i64;
        let volume_total = order.volume_total as i64;
        let r = sqlx::query!("INSERT OR IGNORE INTO market_orders (buy_order, issued, expiry, order_id, item_id, system_id, volume_remain, volume_total) VALUES (?,?,?,?,?,?,?,?)", 
            order.is_buy_order, order.issued, expiry, order_id, type_id, system_id, volume_remain, volume_total).execute(&mut transaction).await;

        match r {
            Ok(_) => {}
            Err(_) => {
                // println!("Error inserting order: {:?}, {:?}", e, order);
            }
        }
    }

    println!("Inserted orders for region: {}", region);

    transaction.commit().await
}
