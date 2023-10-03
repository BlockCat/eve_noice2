use crate::esi::errors::EsiError;

mod update_orders;
mod update_history;

pub use update_orders::update_order_for_region;
pub use update_history::update_history_for_region;

#[derive(Debug)]
pub enum UpdateError {
    MarketHistorySql(sqlx::Error, usize),
    MarketHistoryEsi(EsiError, usize),
    UpdateOrderSql(sqlx::Error, usize),
    UpdateOrderEsi(EsiError, usize),
}
