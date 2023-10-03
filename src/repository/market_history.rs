use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use futures::TryStreamExt;
use sqlx::{sqlite::SqliteRow, Connection, Row, SqlitePool};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

pub struct MarketHistoryRepository(Arc<Mutex<SqlitePool>>);

impl MarketHistoryRepository {
    pub fn new(pool: Arc<Mutex<SqlitePool>>) -> Self {
        Self(pool)
    }
}

impl Clone for MarketHistoryRepository {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl MarketHistoryRepository {
    pub async fn latest_histories(
        &mut self,
        region_id: usize,
    ) -> Result<HashMap<usize, DateTime<Utc>>, sqlx::Error> {
        let lock = self.0.lock().await;
        let mut connection = lock.acquire().await?;

        log::trace!("Querying latest histories for region: {}", region_id);

        let latest_histories = sqlx::query("SELECT item_id, MAX(date) as date from market_history WHERE region_id = ? GROUP BY item_id")
        .bind(region_id as i64)
        .map(|r: SqliteRow| {
            let item_id: i64 = r.try_get("item_id").unwrap();
            let date: NaiveDate = r.try_get("date").unwrap();

            let date = Utc.from_utc_datetime(&date.and_hms_opt(11, 0, 0).unwrap());

            (item_id as usize, date)
        })
        .fetch(connection.as_mut())
        .try_collect::<HashMap<_, _>>()        
        .await?;

        log::trace!("Queried latest histories for region: {}", region_id);

        drop(connection);
        drop(lock);

        Ok(latest_histories)
    }

    pub async fn insert_items(
        &mut self,
        added: Vec<(usize, crate::esi::models::MarketRegionHistoryItem)>,
        region_id: usize,
    ) -> Result<(), sqlx::Error> {
        let lock = self.0.lock().await;
        let mut connection = lock.acquire().await?;
        let mut transaction = connection.begin().await?;

        for (item_id, history) in added {
            let item_id = item_id as i64;
            let region_id = region_id as i64;
            sqlx::query!("INSERT OR REPLACE INTO market_history (date, item_id, region_id, low_price, high_price, average_price, order_count, volume) VALUES (?, ?, ?, ?, ?, ?, ?, ?)", 
            history.date, item_id, region_id, history.lowest, history.highest, history.average, history.order_count, history.volume
        ).execute(transaction.as_mut()).await.map_err(|e| {
            log::error!("Failed to insert history: {:?}. tid: {}, rid: {}", e, item_id, region_id);
            e
        })?;
        }
        transaction.commit().await?;

        Ok(())
    }
}
