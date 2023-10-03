use crate::{
    actions::update_history_for_region,
    repository::{ItemRepository, MarketHistoryRepository},
};
use actix::{Actor, Context, Handler};


use super::StartActor;

pub struct MarketHistoryActor {
    pub region_id: usize,
    pub market_history_repository: MarketHistoryRepository,
    pub item_repository: ItemRepository,

    handle: Option<tokio::task::JoinHandle<()>>,
}

impl MarketHistoryActor {
    pub fn new(
        region_id: usize,
        market_history_repository: MarketHistoryRepository,
        item_repository: ItemRepository,
    ) -> Self {
        Self {
            region_id,
            market_history_repository,
            item_repository,
            handle: None,
        }
    }
}

impl Actor for MarketHistoryActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        log::debug!("MarketHistoryActor created for region: {}", self.region_id);
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> actix::Running {
        log::debug!("MarketHistoryActor stopping for region: {}", self.region_id);
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
        actix::Running::Stop
    }
}

impl Handler<StartActor> for MarketHistoryActor {
    type Result = ();

    fn handle(&mut self, _: StartActor, _ctx: &mut Self::Context) -> Self::Result {
        log::trace!("MarketHistoryActor received StartActor message");
        if let Some(handle) = &self.handle {
            if !handle.is_finished() {
                log::warn!(
                    "MarketHistoryActor already running for region: {}",
                    self.region_id
                );
                return;
            }
        }
        log::debug!("MarketHistoryActor starting for region: {}", self.region_id);
        let region_id = self.region_id;
        let market_history_repository = self.market_history_repository.clone();
        let item_repository = self.item_repository.clone();

        let handle = tokio::spawn(async move {
            match update_history_for_region(region_id, market_history_repository, item_repository)
                .await
            {
                Ok(_) => log::info!("MarketHistoryActor finished for region: {}", region_id),
                Err(e) => log::error!(
                    "MarketHistoryActor failed for region: {}, {:?}",
                    region_id,
                    e
                ),
            }
        });
        self.handle = Some(handle);
    }
}
