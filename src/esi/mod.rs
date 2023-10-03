use self::errors::EsiError;
use futures::future::try_join_all;
use reqwest::{Client, StatusCode};

pub mod errors;
pub mod models;

const BASE_URL: &str = "https://esi.evetech.net/latest";
const USER_AGENT: &str = "EveMarketData/0.1 (zinoonomiwo@gmail.com)";

pub fn create_client() -> Client {
    Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .expect("Failed to create client")
}

pub async fn get_market_history(
    client: &Client,
    region: usize,
    type_id: usize,
) -> Result<Vec<models::MarketRegionHistoryItem>, EsiError> {
    if !is_published(client, type_id).await? {
        return Err(EsiError::NotPublished(type_id));
    }

    let path = format!(
        "{}/markets/{}/history/?type_id={}",
        BASE_URL, region, type_id
    );

    let response = client
        .get(path)
        .send()
        .await
        .map_err(|e| EsiError::MarketHistory(e, region, type_id))?;

    if response.status().as_u16() == 420 {
        return Err(EsiError::ErrorLimited);
    }

    response
        .json::<models::MarketRegionHistory>()
        .await
        .map_err(|e| EsiError::MarketHistory(e, region, type_id))
}

pub async fn get_market_orders(
    client: &Client,
    region: usize,
) -> Result<Vec<models::MarketRegionOrdersItem>, EsiError> {
    let path = format!("{}/markets/{}/orders/?page=1", BASE_URL, region);
    let response = client
        .get(path)
        .send()
        .await
        .map_err(|e| EsiError::MarketOrder(e, region, 1))?;

    if response.status().as_u16() == 420 {
        return Err(EsiError::ErrorLimited);
    }

    let pages = match extract_pages(&response) {
        Ok(pages) => pages,
        Err(e) => {
            return Err(e);
        }
    };

    let mut orders = response
        .json::<models::MarketRegionOrders>()
        .await
        .map_err(|e| EsiError::MarketOrder(e, region, 1))?;

    let paged_orders =
        try_join_all((2..pages).map(|page| get_market_orders_page(client, region, page)))
            .await
            .map(|x| x.into_iter().flatten().collect::<Vec<_>>())?;

    orders.extend(paged_orders);

    Ok(orders)
}

async fn get_market_orders_page(
    client: &Client,
    region: usize,
    page: usize,
) -> Result<Vec<models::MarketRegionOrdersItem>, EsiError> {
    let path = format!("{}/markets/{}/orders/?page={}", BASE_URL, region, page);
    let response = client
        .get(path)
        .send()
        .await
        .map_err(|e| EsiError::MarketOrder(e, region, page))?;

    if response.status().as_u16() == 420 {
        return Err(EsiError::ErrorLimited);
    }

    response
        .json::<models::MarketRegionOrders>()
        .await
        .map_err(|e| EsiError::MarketOrder(e, region, page))
}

pub async fn get_market_region_types(client: &Client, region: usize) -> Result<Vec<i32>, EsiError> {
    let path = format!("{}/markets/{}/types/?page=1", BASE_URL, region);
    let response = client
        .get(path)
        .send()
        .await
        .map_err(|e| EsiError::MarketRegionType(e, region, 1))?;
    let pages = extract_pages(&response)?;

    if response.status().as_u16() == 420 {
        return Err(EsiError::ErrorLimited);
    }

    let mut types = response
        .json::<models::MarketRegionTypes>()
        .await
        .map_err(|e| EsiError::MarketRegionType(e, region, 1))?;

    let paged_types =
        try_join_all((2..pages).map(|page| get_market_region_types_page(client, region, page)))
            .await
            .map(|x| x.into_iter().flatten().collect::<Vec<_>>())?;

    types.extend(paged_types);

    types.sort();

    Ok(types)
}

async fn get_market_region_types_page(
    client: &Client,
    region: usize,
    page: usize,
) -> Result<Vec<i32>, EsiError> {
    let path = format!(
        "{}/markets/{}/types/?datasource=tranquility&page={}",
        BASE_URL, region, page
    );
    let response = client
        .get(path)
        .send()
        .await
        .map_err(|e| EsiError::MarketRegionType(e, region, page))?;

    if response.status().as_u16() == 420 {
        return Err(EsiError::ErrorLimited);
    }

    response
        .json::<models::MarketRegionTypes>()
        .await
        .map_err(|e| EsiError::MarketRegionType(e, region, page))
}

pub async fn is_published(client: &Client, type_id: usize) -> Result<bool, EsiError> {
    let path = format!("{}/universe/types/{}/", BASE_URL, type_id);
    let response = client
        .get(path)
        .send()
        .await
        .map_err(|e| EsiError::PublishCheck(e, type_id))?;

    if response.status().as_u16() == 420 {
        return Err(EsiError::ErrorLimited);
    }
    response
        .json::<models::UniverseTypeId>()
        .await
        .map(|x| x.published)
        .map_err(|e| EsiError::PublishCheck(e, type_id))
}

fn extract_pages(response: &reqwest::Response) -> Result<usize, EsiError> {
    if response.status() == StatusCode::OK {
        let pages = response
            .headers()
            .get("x-pages")
            .ok_or(EsiError::NoPages)?
            .to_str()
            .unwrap()
            .parse::<usize>()
            .unwrap();
        Ok(pages)
    } else {
        return Err(EsiError::ErrorResponse);
    }
}
