use crate::esi::models::MarketRegionOrdersItem;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;

const CHUNK_SIZE: usize = 1000;

pub struct MarketOrderRepository(Arc<Mutex<SqlitePool>>);

impl MarketOrderRepository {
    pub fn new(pool: Arc<Mutex<SqlitePool>>) -> Self {
        Self(pool)
    }
}

impl Clone for MarketOrderRepository {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl MarketOrderRepository {
    pub async fn insert_active_items(
        &mut self,
        items: Vec<MarketRegionOrdersItem>,
    ) -> Result<(), sqlx::Error> {
        let lock = self.0.lock().await;
        let active_order_ids: String = items
            .iter()
            .map(|o| o.order_id.to_string())
            .collect::<Vec<_>>()
            .join(",");

        for batch in items.chunks(CHUNK_SIZE) {
            let mut transaction = lock.begin().await?;
            for order in batch {
                let expiry = order.issued + chrono::Duration::days(order.duration as i64);
                let order_id = order.order_id as i64;
                let type_id = order.type_id as i64;
                let system_id = order.system_id as i64;
                let volume_remain = order.volume_remain as i64;
                let volume_total = order.volume_total as i64;
                let price = order.price as f32;
                sqlx::query!("INSERT OR IGNORE INTO market_orders (buy_order, issued, expiry, order_id, item_id, system_id, volume_remain, volume_total, price) VALUES (?,?,?,?,?,?,?,?, ?)", 
                    order.is_buy_order, order.issued, expiry, order_id, type_id, system_id, volume_remain, volume_total, price).execute(transaction.as_mut()).await
                    .map_err(|e| {
                        log::error!("Failed to insert order: {:?}. tid: {}, sid: {}", e.to_string(), type_id, system_id);
                        e
                    })?;
            }
        }

        let mut connection = lock.acquire().await?;

        sqlx::query(&format!(
            "UPDATE market_orders SET active = 0 WHERE active = 1 AND order_id NOT IN({})",
            active_order_ids
        ))
        .execute(connection.as_mut())
        .await?;

        drop(connection);

        Ok(())
    }
}
