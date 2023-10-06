use actix::{Actor, Addr};
use actix_web::{web, App, HttpServer};
use actors::{MarketHistoryActor, MarketOrderActor, UpdateScheduler};
use esi::EsiClient;
use log::LevelFilter;
use repository::{ItemRepository, MarketHistoryRepository, MarketOrderRepository};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::{sync::Arc, time::Duration};
use tokio::{sync::Mutex, task::JoinHandle};

mod actions;
mod actors;
mod config;
mod esi;
mod eve_auth;
mod repository;

pub struct ActixHandle(Arc<JoinHandle<()>>);

impl Clone for ActixHandle {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl ActixHandle {
    pub fn new(handle: JoinHandle<()>) -> Self {
        Self(Arc::new(handle))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::builder()
        .filter(Some("noice"), LevelFilter::Trace)
        .filter(None, LevelFilter::Info)
        .init();

    EsiClient::new(20);

    let pool = load_sqlite().await;

    let market_history_repository =
        MarketHistoryRepository::new(Arc::new(Mutex::new(pool.clone())));
    let item_repository = ItemRepository::new(Arc::new(Mutex::new(pool.clone())));
    let market_order_repository = MarketOrderRepository::new(Arc::new(Mutex::new(pool.clone())));

    let _system = start_actors(
        market_history_repository.clone(),
        item_repository.clone(),
        market_order_repository.clone(),
    )
    .await;

    HttpServer::new(move || {
        let mhr = market_history_repository.clone();
        let ir = item_repository.clone();
        let mor = market_order_repository.clone();
        App::new()
            .app_data(web::Data::new(mhr))
            .app_data(web::Data::new(ir))
            .app_data(web::Data::new(mor))
            .app_data(web::Data::new(pool.clone()))

        // .service(factory)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?;

    _system.abort();

    Ok(())
}

async fn load_sqlite() -> SqlitePool {
    let sqlite_path = std::env::var("DATABASE_URL").unwrap_or("sqlite:database.db".to_string());

    log::info!("Reading sqlite path: {}", sqlite_path);

    let pool = SqlitePoolOptions::new()
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Some(Duration::from_secs(30)))
        .max_lifetime(Some(Duration::from_secs(200)))
        .connect(&sqlite_path)
        .await
        .unwrap();

    pool
}

async fn start_actors(
    market_history_repository: MarketHistoryRepository,
    item_repository: ItemRepository,
    market_order_repository: MarketOrderRepository,
) -> tokio::task::JoinHandle<ActorHolder> {
    actix::spawn(async move {
        let history_actors = actors::load_market_history_actors(
            &[10000002, 10000043],
            market_history_repository,
            item_repository.clone(),
        );

        let order_actors = actors::load_market_order_actors(
            &[10000002, 10000043],
            market_order_repository,
            item_repository.clone(),
        );

        // history_actors.iter().for_each(|s| s.do_send(StartActor));
        // order_actors.iter().for_each(|s| s.do_send(StartActor));

        let history_scheduler = actors::UpdateScheduler::new(
            "0 0 12 * * * *".to_string(),
            history_actors
                .iter()
                .map(|x| x.clone().recipient())
                .collect(),
        );
        let order_scheduler = actors::UpdateScheduler::new(
            "0 */6 * * * * *".to_string(),
            order_actors.iter().map(|x| x.clone().recipient()).collect(),
        );

        let history_actors = MarketHistoryActors(history_actors);
        let order_actors = MarketOrderActors(order_actors);

        ActorHolder {
            _history_actors: history_actors,
            _order_actors: order_actors,
            _history_scheduler: history_scheduler.start(),
            _order_scheduler: order_scheduler.start(),
        }
    })
}

pub struct MarketHistoryActors(Vec<Addr<MarketHistoryActor>>);
pub struct MarketOrderActors(Vec<Addr<MarketOrderActor>>);

pub struct ActorHolder {
    _history_actors: MarketHistoryActors,
    _order_actors: MarketOrderActors,
    _history_scheduler: Addr<UpdateScheduler>,
    _order_scheduler: Addr<UpdateScheduler>,
}
