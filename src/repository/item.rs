use std::{collections::HashSet, sync::Arc};

use sqlx::SqlitePool;
use tokio::sync::Mutex;

pub struct ItemRepository(Arc<Mutex<SqlitePool>>);

impl Clone for ItemRepository {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl ItemRepository {
    pub fn new(pool: Arc<Mutex<SqlitePool>>) -> Self {
        Self(pool)
    }

    pub async fn tradeable_item_ids(&mut self) -> Result<HashSet<usize>, sqlx::Error> {
        let lock = self.0.lock().await;
        let mut connection = lock.acquire().await?;

        let all_items = sqlx::query!(
            "SELECT id FROM eve_items WHERE published = 1 AND market_group_id IS NOT NULL"
        )
        .fetch_all(connection.as_mut())
        .await?
        .into_iter()
        .map(|x| x.id as usize)
        .collect::<HashSet<_>>();

        drop(connection);
        drop(lock);

        Ok(all_items)
    }
}
