use super::StartActor;
use crate::{actions::update_order_for_region, repository::MarketOrderRepository};
use actix::{Actor, Context, Handler};
use tokio;

pub struct MarketOrderActor {
    pub region_id: usize,
    pub market_order_repository: MarketOrderRepository,

    handle: Option<tokio::task::JoinHandle<()>>,
}

impl MarketOrderActor {
    pub fn new(region_id: usize, market_order_repository: MarketOrderRepository) -> Self {
        Self {
            region_id,
            market_order_repository,
            handle: None,
        }
    }
}

impl Actor for MarketOrderActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        log::debug!("MarketOrderActor created for region: {}", self.region_id);
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> actix::Running {
        log::debug!("MarketOrderActor stopping for region: {}", self.region_id);
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
        actix::Running::Stop
    }
}

impl Handler<StartActor> for MarketOrderActor {
    type Result = ();

    fn handle(&mut self, _: StartActor, _ctx: &mut Self::Context) -> Self::Result {
        log::trace!("MarketOrderActor received StartActor message");
        if let Some(handle) = &self.handle {
            if !handle.is_finished() {
                log::warn!(
                    "MarketOrderActor already running for region: {}",
                    self.region_id
                );
                return;
            }
        }
        log::debug!("MarketOrderActor starting for region: {}", self.region_id);
        let region_id = self.region_id.clone();
        let market_order_repository = self.market_order_repository.clone();

        let handle = tokio::spawn(async move {
            match update_order_for_region(region_id, market_order_repository).await {
                Ok(_) => log::info!("MarketOrderActor finished for region: {}", region_id),
                Err(e) => log::error!("MarketOrderActor failed for region: {}, {:?}", region_id, e),
            }
        });
        self.handle = Some(handle);
    }
}
