use config::AppConfig;
use rocket::fairing::AdHoc;
use rocket_db_pools::Database;

#[macro_use]
extern crate rocket;

mod actions;
mod config;
mod esi;
mod eve_auth;
// mod eve_esi;
mod actors;
mod jobs;
mod repository;

#[derive(Clone, Database)]
#[database("sqlite_eve")]
struct EveDatabase(sqlx::SqlitePool);

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[launch]
async fn rocket() -> _ {
    rocket::build()
        .attach(AdHoc::config::<AppConfig>())
        .attach(EveDatabase::init())
        .attach(jobs::NoiceJobScheduler::new().await)
        .mount("/", routes![index])

    // .mount(
    //     "/auth",
    //     routes![eve_auth::eve_login, eve_auth::eve_callback],
    // )
}
