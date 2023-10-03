// use std::thread;

// use crate::EveDatabase;
// use clokwerk::{Job, Scheduler, TimeUnits};
// use rocket::{
//     fairing::{Fairing, Info, Kind},
//     tokio::{self, task::spawn_blocking},
//     Orbit, Rocket,
// };
// use rocket_db_pools::Database;

// pub struct MarketHistoryUpdater {}
// pub struct MarketOrderUpdater {}

// impl MarketHistoryUpdater {
//     pub fn new() -> Self {
//         MarketHistoryUpdater {}
//     }
// }

// impl MarketOrderUpdater {
//     pub fn new() -> Self {
//         MarketOrderUpdater {}
//     }
// }

// #[rocket::async_trait]
// impl Fairing for MarketHistoryUpdater {
//     fn info(&self) -> Info {
//         Info {
//             name: "Market History Updater",
//             kind: Kind::Liftoff,
//         }
//     }
//     async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
//         let db = EveDatabase::fetch(&rocket)
//             .expect("Could not get database")
//             .clone();
//         let db2 = EveDatabase::fetch(&rocket)
//             .expect("Could not get database")
//             .clone();

//         let mut scheduler = Scheduler::new();
//         scheduler.every(1.day()).at("12:00:00").run(move || {
//             let db = db.clone();
//             tokio::spawn(async move {
//                 let pool = db.acquire().await.unwrap();
//                 crate::actions::update_orders::update_history_for_region(10000002, pool).await;
//                 let pool = db.acquire().await.unwrap();
//                 crate::actions::update_orders::update_history_for_region(10000043, pool).await;
//             });
//         });

//         match spawn_blocking(|| async move {
//             loop {
//                 scheduler.run_pending();
//                 thread::sleep(std::time::Duration::from_millis(1000));
//             }
//         })
//         .await
//         {
//             Ok(c) => c.await,
//             Err(e) => println!("Error: {:?}", e),
//         }
//     }
// }

// #[rocket::async_trait]
// impl Fairing for MarketOrderUpdater {
//     fn info(&self) -> Info {
//         Info {
//             name: "Market Order Updater",
//             kind: Kind::Liftoff,
//         }
//     }
//     async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
//         let db = EveDatabase::fetch(&rocket)
//             .expect("Could not get database")
//             .clone();
//         let db2 = db.clone();
//         let mut scheduler = Scheduler::new();

//         scheduler.every(30.minutes()).run(move || {
//             println!("Starting update");
//             let db = db.clone();
//             tokio::spawn(async move {
//                 let pool = db.acquire().await.unwrap();
//                 crate::actions::update_orders::update_order_for_region(10000002, pool)
//                     .await
//                     .unwrap();
//                 let pool = db.acquire().await.unwrap();
//                 crate::actions::update_orders::update_order_for_region(10000043, pool)
//                     .await
//                     .unwrap();
//             });
//         });

//         match spawn_blocking(|| async move {
//             let pool = db2.acquire().await.unwrap();
//             crate::actions::update_orders::update_order_for_region(10000002, pool)
//                 .await
//                 .unwrap();
//             let pool = db2.acquire().await.unwrap();
//             crate::actions::update_orders::update_order_for_region(10000043, pool)
//                 .await
//                 .unwrap();
//             loop {
//                 scheduler.run_pending();
//                 thread::sleep(std::time::Duration::from_millis(1000));
//             }
//         })
//         .await
//         {
//             Ok(c) => c.await,
//             Err(w) => println!("Error: {:?}", w),
//         }
//     }
// }
