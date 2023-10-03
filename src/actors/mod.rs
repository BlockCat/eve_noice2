use self::market_history_actor::MarketHistoryActor;
use crate::repository::MarketHistoryRepository;
use actix::{Actor, Addr, Message, Recipient};

pub use market_history_actor::MarketHistoryMessage;

mod market_history_actor;

#[derive(Message)]
#[rtype(result = "()")]
pub struct StartActor;

pub fn load_market_history_actors(
    regions: &[usize],
    recipient: Recipient<MarketHistoryMessage>,
    repository: MarketHistoryRepository,
) -> Vec<Addr<MarketHistoryActor>> {
    regions
        .iter()
        .map(|region_id| {
            let actor = MarketHistoryActor::new(*region_id, recipient.clone(), repository.clone());
            let addr = actor.start();
            addr
        })
        .collect()
}
