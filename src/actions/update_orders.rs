use super::UpdateError;
use crate::{
    esi::{get_market_orders, EsiClient},
    repository::{ItemRepository, MarketOrderRepository},
};

pub async fn update_order_for_region(
    region_id: usize,
    mut market_order_repository: MarketOrderRepository,
    mut item_repository: ItemRepository,
) -> Result<(), UpdateError> {
    let client = EsiClient::current();

    log::debug!("Starting orders for region: {}", region_id);

    let all_items = item_repository
        .tradeable_item_ids()
        .await
        .map_err(|e| UpdateError::UpdateOrderSql(e, region_id))?;

    let orders = get_market_orders(client.clone(), region_id)
        .await
        .map_err(|e| UpdateError::UpdateOrderEsi(e, region_id))?
        .into_iter()
        .filter(|x| all_items.contains(&(x.type_id as usize)))
        .collect::<Vec<_>>();

    log::debug!("Region: {}, orders: {}", region_id, orders.len());

    market_order_repository
        .insert_active_items(orders, region_id)
        .await
        .map_err(|e| UpdateError::UpdateOrderSql(e, region_id))?;

    log::debug!("Inserted orders for region: {}", region_id);

    Ok(())
}
