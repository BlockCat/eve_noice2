use actix::{Actor, Addr};
use actix_web::{web, App, HttpServer};
use actors::{StartActor, UpdateScheduler};
use log::{LevelFilter, Metadata, Record};
use repository::{ItemRepository, MarketHistoryRepository, MarketOrderRepository};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::sync::Arc;
use tokio::{sync::Mutex, task::JoinHandle};

mod actions;
mod actors;
mod config;
mod esi;
mod eve_auth;
mod repository;

static LOGGER: SimpleLogger = SimpleLogger;

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
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Debug))
        .unwrap();

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
    })
    .bind(("127.0.0.1", 8080))?
    .workers(2)
    .run()
    .await
}

async fn load_sqlite() -> SqlitePool {
    SqlitePoolOptions::new()
        .connect("sqlite:database.db")
        .await
        .unwrap()
}

async fn start_actors(
    market_history_repository: MarketHistoryRepository,
    item_repository: ItemRepository,
    market_order_repository: MarketOrderRepository,
) -> tokio::task::JoinHandle<(Addr<UpdateScheduler>, Addr<UpdateScheduler>)> {
    actix::spawn(async move {
        let history_actors = actors::load_market_history_actors(
            &[10000002, 10000043],
            market_history_repository,
            item_repository,
        );

        let order_actors =
            actors::load_market_order_actors(&[10000002, 10000043], market_order_repository);

        history_actors.iter().for_each(|s| s.do_send(StartActor));
        order_actors.iter().for_each(|s| s.do_send(StartActor));

        let history_scheduler = actors::UpdateScheduler::new(
            "0 0 12 * * * *".to_string(),
            history_actors
                .iter()
                .map(|x| x.clone().recipient())
                .collect(),
        );
        let order_scheduler = actors::UpdateScheduler::new(
            "0 */30 * * * * *".to_string(),
            order_actors.iter().map(|x| x.clone().recipient()).collect(),
        );

        (history_scheduler.start(), order_scheduler.start())
    })
}

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        // metadata.level() <= Level::Info
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}