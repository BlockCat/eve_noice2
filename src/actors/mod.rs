use crate::repository::{ItemRepository, MarketHistoryRepository, MarketOrderRepository};
use actix::{Actor, Addr, Message};

pub use market_history_actor::MarketHistoryActor;
pub use market_order_actor::MarketOrderActor;
pub use update_scheduler::UpdateScheduler;

mod market_history_actor;
mod market_order_actor;
mod update_scheduler;

#[derive(Message)]
#[rtype(result = "()")]
pub struct StartActor;

pub fn load_market_history_actors(
    regions: &[usize],
    market_history_repository: MarketHistoryRepository,
    item_repository: ItemRepository,
) -> Vec<Addr<MarketHistoryActor>> {
    regions
        .iter()
        .map(|region_id| {
            let actor = MarketHistoryActor::new(
                *region_id,
                market_history_repository.clone(),
                item_repository.clone(),
            );

            actor.start()
        })
        .collect()
}

pub fn load_market_order_actors(
    regions: &[usize],
    market_order_repository: MarketOrderRepository,
    item_repository: ItemRepository,
) -> Vec<Addr<MarketOrderActor>> {
    regions
        .iter()
        .map(|region_id| {
            let actor = MarketOrderActor::new(
                *region_id,
                market_order_repository.clone(),
                item_repository.clone(),
            );

            actor.start()
        })
        .collect()
}
