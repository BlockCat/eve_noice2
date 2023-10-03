use super::UpdateError;
use crate::{
    esi::{create_client, get_market_orders},
    repository::MarketOrderRepository,
};

pub async fn update_order_for_region(
    region_id: usize,
    mut market_order_repository: MarketOrderRepository,
) -> Result<(), UpdateError> {
    let client = create_client();

    log::debug!("Starting orders for region: {}", region_id);

    let orders = get_market_orders(&client, region_id)
        .await
        .map_err(|e| UpdateError::UpdateOrderEsi(e, region_id))?;

    log::debug!("Region: {}, orders: {}", region_id, orders.len());

    market_order_repository
        .insert_active_items(orders)
        .await
        .map_err(|e| UpdateError::UpdateOrderSql(e, region_id))?;

    log::debug!("Inserted orders for region: {}", region_id);

    Ok(())
}
