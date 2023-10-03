use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use rocket::tokio::sync::Mutex;
use sqlx::{Row, SqlitePool};
use std::{collections::HashMap, sync::Arc};

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

        let latest_histories = sqlx::query("SELECT DISTINCT id, item_id, MAX(date) as date from market_history WHERE region_id = ? GROUP BY item_id")
        .bind(region_id as i64)
        .fetch_all(connection.as_mut())
        .await?;

        drop(connection);
        drop(lock);

        let latest_histories = latest_histories
            .into_iter()
            .map(|row| {
                let item_id: i64 = row.try_get("item_id").unwrap();
                let date: NaiveDate = row.try_get("date").unwrap();

                let date = Utc.from_utc_datetime(&date.and_hms_opt(11, 0, 0).unwrap());

                (item_id as usize, date)
            })
            .collect::<HashMap<usize, DateTime<Utc>>>();
        Ok(latest_histories)
    }
}
