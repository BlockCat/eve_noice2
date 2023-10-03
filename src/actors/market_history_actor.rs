use crate::repository::MarketHistoryRepository;
use actix::{Actor, Context, Handler, Message, Recipient};
use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};
use rocket::tokio;

use super::StartActor;

#[derive(Message)]
#[rtype(result = "Result<(), ()>")]
pub struct MarketHistoryMessage {
    pub region_id: usize,
}

pub struct MarketHistoryActor {
    pub region_id: usize,
    pub market_history_recipient: Recipient<MarketHistoryMessage>,
    pub market_history_repository: MarketHistoryRepository,

    handle: Option<tokio::task::JoinHandle<()>>,
}

impl MarketHistoryActor {
    pub fn new(
        region_id: usize,
        market_history_recipient: Recipient<MarketHistoryMessage>,
        market_history_repository: MarketHistoryRepository,
    ) -> Self {
        Self {
            region_id,
            market_history_recipient,
            market_history_repository,
            handle: None,
        }
    }
}

impl Actor for MarketHistoryActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("MarketHistoryActor created for region: {}", self.region_id);
    }

    fn stopping(&mut self, ctx: &mut Self::Context) -> actix::Running {
        println!("MarketHistoryActor stopping for region: {}", self.region_id);
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
        actix::Running::Stop
    }
}

impl Handler<StartActor> for MarketHistoryActor {
    type Result = ();

    fn handle(&mut self, _: StartActor, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(handle) = &self.handle {
            if !handle.is_finished() {
                return;
            }
        }
        println!("MarketHistoryActor starting for region: {}", self.region_id);
        let region_id = self.region_id.clone();
        let recipient = self.market_history_recipient.clone();
        let mut repository = self.market_history_repository.clone();

        let handle = tokio::spawn(async move {
            let latest_histories = match repository.latest_histories(region_id).await {
                Ok(histories) => histories,
                Err(_) => {
                    println!("MarketHistoryActor failed to get latest histories");
                    return;
                }
            };

            let result = recipient.do_send(MarketHistoryMessage {
                region_id: region_id,
            });

            println!("MarketHistoryActor finished for region: {}", region_id);
            match result {
                Ok(_) => {}
                Err(_) => println!("MarketHistoryActor failed to send message"),
            }
        });
        self.handle = Some(handle);
    }
}

/// Returns the latest market data that is available.
/// New market data of the previous day is available at 11:05 UTC.
fn current_latest_market_data() -> DateTime<Utc> {
    let today = Utc::now();
    if today.hour() < 11 {
        Utc.with_ymd_and_hms(today.year(), today.month(), today.day() - 2, 11, 0, 0)
            .unwrap()
    } else {
        Utc.with_ymd_and_hms(today.year(), today.month(), today.day() - 1, 11, 0, 0)
            .unwrap()
    }
}
