use std::collections::HashMap;

use actix_web::{get, web, HttpResponse, Responder, Result};
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{
    cache::Cache,
    repository::{MarketHistoryAverage, MarketHistoryRepository, MarketOrderRepository},
};

#[derive(Deserialize)]
struct MarginTableQuery {
    region: Option<usize>,
    page: Option<usize>,
    margin: Option<f64>,
    min_volume: Option<usize>,
}

#[get("/")]
async fn margin_table(
    query: web::Query<MarginTableQuery>,
    order_repository: web::Data<MarketOrderRepository>,
    history_repository: web::Data<MarketHistoryRepository>,
    sqlx_pool: web::Data<SqlitePool>,
) -> Result<impl Responder> {
    let region_id = query.region.unwrap_or(10000002);
    let page = query.page.unwrap_or(0);
    let min_margin = query.margin.unwrap_or(0.2);
    let min_volume = query.min_volume.unwrap_or(3);

    let page_size = 100;
    let start = page * page_size;
    let end = (page + 1) * page_size;

    // We have:
    // - Lowest sell price 7d + 30d
    // - Highest buy price 7d + 30d
    // - Average price 7d + 30d
    // - Volume traded 7d + 30d
    // - ?Competition? (number of orders)

    // Parts the score should be based on:
    // I think it should be a daily profit score averaged of 7d and 30d, so:
    // - Profit per item (highest buy price * 1.09 - lowest sell price * 0.91)
    //      - Taxes (buy/sell)
    //      - Broker fees (buy/sell)
    // - Volume of items that the user captures
    //      - Competition
    //      - Volume traded

    // We also need a reliability score

    // let buy_order_prices = order_repository
    //     .region_buy_prices(region_id)
    //     .await
    //     .map_err(|e| {
    //         log::error!("Could not read buy order prices: {}", e);
    //         actix_web::error::ErrorInternalServerError("Could not read buy order prices")
    //     })?;

    // let sell_order_prices = order_repository
    //     .region_sell_prices(region_id)
    //     .await
    //     .map_err(|e| {
    //         log::error!("Could not read sell order prices: {}", e);
    //         actix_web::error::ErrorInternalServerError("Could not read sell order prices")
    //     })?;

    let averages = history_repository.averages().await.map_err(|e| {
        log::error!("Could not read averages: {}", e);
        actix_web::error::ErrorInternalServerError("Could not read averages")
    })?;

    let item_names = sqlx::query!("SELECT id, name FROM eve_items")
        .map(|row| (row.id as usize, row.name))
        .fetch(sqlx_pool.get_ref())
        .try_collect::<HashMap<_, _>>()
        .await
        .map_err(|e| {
            log::error!("Could not read item names: {}", e);
            actix_web::error::ErrorInternalServerError("Could not read item names")
        })?;

    let avg_days = 3;

    let buy_competition = order_repository
        .region_buy_competition(region_id, 24 * avg_days)
        .await
        .map_err(|e| {
            log::error!("Could not read buy competition: {}", e);
            actix_web::error::ErrorInternalServerError("Could not read buy competition")
        })?;

    let sell_competition = order_repository
        .region_sell_competition(region_id, 24 * avg_days)
        .await
        .map_err(|e| {
            log::error!("Could not read sell competition: {}", e);
            actix_web::error::ErrorInternalServerError("Could not read sell competition")
        })?;

    // let buy_volume = order_repository
    //     .region_confirmed_buy_volume(region_id, 24 * avg_days)
    //     .await
    //     .map_err(|e| {
    //         log::error!("Could not read buy volume: {}", e);
    //         actix_web::error::ErrorInternalServerError("Could not read buy volume")
    //     })?;
    // let sell_volume = order_repository
    //     .region_confirmed_sell_volume(region_id, 24 * avg_days)
    //     .await
    //     .map_err(|e| {
    //         log::error!("Could not read sell volume: {}", e);
    //         actix_web::error::ErrorInternalServerError("Could not read sell volume")
    //     })?;

    let mut items = Vec::new();

    for (item_id, average) in averages {
        let name = item_names.get(&item_id).unwrap().clone();

        let buy_competition = *buy_competition.get(&item_id).unwrap_or(&0) / avg_days;
        let sell_competition = *sell_competition.get(&item_id).unwrap_or(&0) / avg_days;

        // let zucht = Vec::new();

        // let buy_volume = buy_volume.get(&item_id).unwrap_or(&zucht);
        // let sell_volume = sell_volume.get(&item_id).unwrap_or(&zucht);

        items.push(MarginRowItem::new(
            item_id,
            name,
            average,
            buy_competition,
            sell_competition,
        ));
    }

    let mut items: Vec<_> = items
        .into_iter()
        .filter(|e| e.margin >= min_margin && e.volume >= min_volume)
        .collect();

    // for buy_order in buy_order_prices {
    //     let item_id = buy_order.0 as usize;
    //     if let Some(sell_order) = sell_order_prices.get(&item_id) {
    //         let name = item_names.get(&item_id).unwrap().clone();

    //         let buy_competition = *buy_competition.get(&item_id).unwrap_or(&0) / avg_days;
    //         let sell_competition = *sell_competition.get(&item_id).unwrap_or(&0) / avg_days;

    //         let zucht = Vec::new();

    //         let buy_volume = buy_volume.get(&item_id).unwrap_or(&zucht);
    //         let sell_volume = sell_volume.get(&item_id).unwrap_or(&zucht);

    //         items.push(MarginRowItem::new(
    //             item_id,
    //             name,
    //             buy_order.1,
    //             *sell_order,
    //             buy_competition,
    //             sell_competition,
    //             buy_volume,
    //             sell_volume,
    //         ));
    //     }
    // }

    items.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap().reverse());

    let end = if end > items.len() { items.len() } else { end };
    let start = if start >= items.len() {
        items.len() - 1
    } else {
        start
    };

    Ok(HttpResponse::Ok().json(&items[start..end]))
}

#[derive(Serialize)]
struct MarginRowItem {
    item_id: usize,
    name: String,
    buy_price: f64,
    sell_price: f64,
    margin: f64,
    spread: f64,
    score: f64,
    volume: usize,
    profit_per_item: f64,
    traded: f64,
    sell_competition: usize,
    buy_competition: usize,
}

impl MarginRowItem {
    fn new(
        item_id: usize,
        name: String,
        average: MarketHistoryAverage,
        buy_competition: usize,
        sell_competition: usize,
    ) -> Self {
        let profit_per_item = (average.avg_high * 1.09) - (average.avg_low * 0.91);
        let spread = average.avg_high - average.avg_low;

        let traded = average.avg_price as f64 * average.avg_volume as f64;

        let captureable = 0.3;

        Self {
            item_id,
            name,
            profit_per_item,
            sell_price: average.avg_high,
            buy_price: average.avg_low,
            margin: spread / average.avg_high,
            volume: average.avg_volume as usize,
            buy_competition,
            sell_competition,
            spread,
            traded,
            score: average.avg_volume / 2.0 * captureable * profit_per_item
                / (buy_competition + sell_competition + 1) as f64,
        }
    }
}
