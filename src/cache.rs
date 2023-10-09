use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Cache<T> {
    value: Option<(DateTime<Utc>, Arc<T>)>,
    duration: Duration,
}

unsafe impl<T> Send for Cache<T> {}
unsafe impl<T> Sync for Cache<T> {}

impl<T> Cache<T> {
    pub fn new(duration: Duration) -> Self {
        Self {
            value: None,
            duration,
        }
    }

    pub fn get(&self) -> Option<&T> {
        if let Some((last_date, value)) = &self.value {
            let now = Utc::now();
            if now - *last_date <= self.duration {
                return Some(value);
            }
        }
        None
    }

    pub fn set(&mut self, value: T) {
        self.value = Some((Utc::now(), Arc::new(value)));
    }
}
