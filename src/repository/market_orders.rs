use crate::esi::models::MarketRegionOrdersItem;
use chrono::format;
use futures::TryStreamExt;
use sqlx::SqlitePool;
use std::{sync::Arc, collections::HashMap, borrow::BorrowMut};
use tokio::sync::Mutex;

const CHUNK_SIZE: usize = 1000;


#[derive(Debug)]
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
        region_id: usize,
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
            transaction.commit().await?;
        }

        let mut connection = lock.acquire().await?;

        sqlx::query(&format!(
            "UPDATE market_orders SET active = 0 WHERE active = 1 AND order_id NOT IN({}) AND system_id IN (SELECT id FROM eve_system WHERE region_id = {})",
            active_order_ids, region_id
        ))        
        .execute(connection.as_mut())
        .await?;

        drop(connection);

        Ok(())
    }

    pub async fn region_sell_prices(&self, region_id: usize) -> Result<HashMap<usize, f64>, sqlx::Error> {
        let lock = self.0.lock().await;
        let mut connection = lock.acquire().await?;
        let region_id = region_id as i64;
        sqlx::query!(
            r#"SELECT item_id, MIN(price) as "sell_price! : f64" FROM market_orders WHERE system_id IN (select id from eve_system where region_id = ?) AND buy_order = 0 AND active = 1 GROUP BY item_id HAVING COUNT(price) > 0"#,
            region_id
        )
        .map(|row| {
            let item_id = row.item_id as usize;
            let price: f64 = row.sell_price;
    
            (item_id, price)
        })
        .fetch(connection.as_mut())
        .try_collect::<HashMap<_,_>>().await
    }

    
    pub async fn region_buy_prices(&self, region_id: usize) -> Result<HashMap<usize, f64>, sqlx::Error> {
        let lock = self.0.lock().await;
        let mut connection = lock.acquire().await?;
        let region_id = region_id as i64;
        sqlx::query!(
            r#"SELECT item_id, MAX(price) as "buy_price! : f64" FROM market_orders WHERE system_id IN (select id from eve_system where region_id = ?) AND buy_order = 1 AND active = 1 GROUP BY item_id HAVING COUNT(price) > 0"#,
            region_id
        )
        .map(|row| {
            let item_id = row.item_id as usize;
            let price: f64 = row.buy_price;
    
            (item_id, price)
        })
        .fetch(connection.as_mut())
        .try_collect::<HashMap<_,_>>().await
    }

    pub async fn region_buy_competition(&self, region_id: usize, last_hours: usize) -> Result<HashMap<usize, usize>, sqlx::Error> {
        let lock = self.0.lock().await;
        let mut connection = lock.acquire().await?;
        let region_id = region_id as i64;
        let past_hours = format!("-{} hours", last_hours);
        sqlx::query!(
            r#"SELECT item_id, count(1) as "competition! : i64" FROM market_orders 
            WHERE system_id IN (select id from eve_system where region_id = ?) AND buy_order = 1 AND datetime(issued) > datetime('now', ?)
            GROUP BY item_id"#,
            region_id, past_hours
        )
        .map(|row| {
            let item_id = row.item_id as usize;
            let price = row.competition as usize;
    
            (item_id, price)
        })
        .fetch(connection.as_mut())
        .try_collect::<HashMap<_,_>>().await
    }

    pub async fn region_sell_competition(&self, region_id: usize, last_hours: usize) -> Result<HashMap<usize, usize>, sqlx::Error> {
        let lock = self.0.lock().await;
        let mut connection = lock.acquire().await?;
        let region_id = region_id as i64;
        let past_hours = format!("-{} hours", last_hours);
        sqlx::query!(
            r#"SELECT item_id, count(1) as "competition! : i64" FROM market_orders 
            WHERE system_id IN (select id from eve_system where region_id = ?) AND buy_order = 0 AND datetime(issued) > datetime('now', ?)
            GROUP BY item_id"#,
            region_id, past_hours
        )
        .map(|row| {
            let item_id = row.item_id as usize;
            let price = row.competition as usize;
    
            (item_id, price)
        })
        .fetch(connection.as_mut())
        .try_collect::<HashMap<_,_>>().await
    }

    pub async fn region_confirmed_buy_volume(&self, region_id: usize, last_hours: usize) -> Result<HashMap<usize, Vec<(f64, usize)>>, sqlx::Error> {
        let lock = self.0.lock().await;
        let mut connection = lock.acquire().await?;        
        let region_id = region_id as i64;
        let last_hours = format!("-{} hours", last_hours);
        let items = sqlx::query!(r#"SELECT order_id, item_id, price, MAX(volume_remain) - MIN(volume_remain) as "fulfilled! : i64" FROM market_orders WHERE system_id IN (select id from eve_system where region_id = ?) AND buy_order=1 AND datetime(created) > datetime('now', ?) GROUP BY order_id, price"#, region_id, last_hours)
        .map(|row| {
            let item_id = row.item_id as usize;
            let price = row.price;
            let volume = row.fulfilled as usize;
            
            (item_id, price, volume)
        })
        .fetch_all(connection.as_mut()).await?;

        let mut map: HashMap<usize, Vec<(f64, usize)>> = HashMap::new();
        for (item_id, price, volume) in items {
            map.entry(item_id).or_default().push((price, volume));
        }

        Ok(map)
    }

    pub async fn region_confirmed_sell_volume(&self, region_id: usize, last_hours: usize) -> Result<HashMap<usize, Vec<(f64, usize)>>, sqlx::Error> {
        let lock = self.0.lock().await;
        let mut connection = lock.acquire().await?;        
        let region_id = region_id as i64;
        let last_hours = format!("-{} hours", last_hours);
        let items = sqlx::query!(r#"SELECT order_id, item_id, price, MAX(volume_remain) - MIN(volume_remain) as "fulfilled! : i64" FROM market_orders WHERE system_id IN (select id from eve_system where region_id = ?) AND buy_order=0 AND datetime(created) > datetime('now', ?) GROUP BY order_id, price"#, region_id, last_hours)
        .map(|row| {
            let item_id = row.item_id as usize;
            let price = row.price;
            let volume = row.fulfilled as usize;
            
            (item_id, price, volume)
        })
        .fetch_all(connection.as_mut()).await?;

        let mut map: HashMap<usize, Vec<(f64, usize)>> = HashMap::new();
        for (item_id, price, volume) in items {
            map.entry(item_id).or_default().push((price, volume));
        }

        Ok(map)
    }
}
