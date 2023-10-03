use crate::{
    actors::{load_market_history_actors, MarketHistoryMessage, StartActor},
    config::AppConfig,
};
use actix::{Actor, Context, Handler};
use actix_rt::{task::spawn_blocking, Arbiter, System};
use rocket::{
    fairing::{Fairing, Info, Kind},
    tokio::sync::Mutex,
    tokio::{
        self,
        task::{spawn_local, JoinHandle, LocalSet},
    },
    Orbit, Rocket,
};
use std::{sync::Arc, thread};

pub struct NoiceJobScheduler(Mutex<Option<JoinHandle<()>>>);

impl NoiceJobScheduler {
    pub async fn new() -> Self {
        NoiceJobScheduler(Mutex::new(None))
    }
}

#[rocket::async_trait]
impl Fairing for NoiceJobScheduler {
    fn info(&self) -> Info {
        Info {
            name: "Job Scheduler",
            kind: Kind::Liftoff,
        }
    }
    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        let config = rocket.state::<AppConfig>().unwrap().clone();
        let db = rocket.state::<crate::EveDatabase>().unwrap().clone();
        let pool = Arc::new(Mutex::new(db.0.clone()));
        let region_ids = config.regions.iter().map(|x| *x.1).collect::<Vec<usize>>();

        actix::spawn(async move {
            let pool = pool.clone();
            let region_ids = region_ids.clone();
            println!("Starting job scheduler with regions: {:?}", region_ids);

            let market_history_repository =
                crate::repository::MarketHistoryRepository::new(pool.clone());

            let db = DatabaseActor.start();
            let ad = db.recipient();

            let actors =
                load_market_history_actors(&region_ids, ad.clone(), market_history_repository);

            for actor in actors {
                actor.do_send(StartActor);
            }
        });
    }
}

struct DatabaseActor;

impl Actor for DatabaseActor {
    type Context = Context<Self>;
}

impl Handler<MarketHistoryMessage> for DatabaseActor {
    type Result = Result<(), ()>;

    fn handle(&mut self, msg: MarketHistoryMessage, ctx: &mut Self::Context) -> Self::Result {
        println!("Received message");
        Ok(())
    }
}
