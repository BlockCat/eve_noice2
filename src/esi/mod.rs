use self::{
    errors::EsiError,
    models::{MarketRegionHistory, UniverseTypeId},
};
use futures::future::try_join_all;
use reqwest::{Client, Response};
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub mod errors;
pub mod models;

const BASE_URL: &str = "https://esi.evetech.net/latest";
const USER_AGENT: &str = "EveMarketData/0.1 (zinoonomiwo@gmail.com)";

static mut ESI_CLIENT: Option<EsiClient> = None;

pub struct EsiClient {
    client: Client,
    semaphore: Arc<Semaphore>,
}

impl Clone for EsiClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            semaphore: self.semaphore.clone(),
        }
    }
}

impl EsiClient {
    pub fn current() -> Self {
        unsafe {
            match ESI_CLIENT {
                Some(ref client) => client.clone(),
                None => panic!("EsiClient not initialized"),
            }
        }
    }
    pub fn new(permits: usize) -> Self {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .gzip(true)
            .use_rustls_tls()
            .connection_verbose(false)
            .pool_max_idle_per_host(permits)
            .build()
            .expect("Failed to create client");

        let client = Self {
            client,
            semaphore: Arc::new(Semaphore::new(permits)),
        };
        unsafe {
            ESI_CLIENT = Some(client.clone());
        }

        client
    }

    pub async fn get_response(&self, path: &str) -> Result<Response, EsiError> {
        let path = format!("{}{}", BASE_URL, path);
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let permit = self
            .semaphore
            .acquire()
            .await
            .expect("Could not acquire permit");
        let response = self
            .client
            .get(path)
            .send()
            .await
            .map_err(EsiError::ConnectionError)?;
        drop(permit);

        if response.status().as_u16() == 420 {
            log::error!("Error limited: {:?}", response);
            return Err(EsiError::ErrorLimited);
        }

        if response.status().as_u16() >= 400 {
            log::error!("Error response: {:?}", response);
            return Err(EsiError::ErrorResponse);
        }

        Ok(response)
    }

    pub async fn get<D: DeserializeOwned>(&self, path: &str) -> Result<D, EsiError> {
        let response = self.get_response(path).await?;
        let data = response.json::<D>().await.map_err(EsiError::JsonError)?;
        Ok(data)
    }
}

pub async fn get_market_history(
    client: EsiClient,
    region: usize,
    type_id: usize,
) -> Result<MarketRegionHistory, EsiError> {
    if !is_published(client.clone(), type_id).await? {
        return Err(EsiError::NotPublished(type_id));
    }

    client
        .get(&format!("/markets/{}/history/?type_id={}", region, type_id))
        .await
}

pub async fn get_market_orders(
    client: EsiClient,
    region: usize,
) -> Result<Vec<models::MarketRegionOrdersItem>, EsiError> {
    let response = client
        .get_response(&format!("/markets/{}/orders/?page=1", region))
        .await?;

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
        try_join_all((2..pages).map(|page| get_market_orders_page(client.clone(), region, page)))
            .await
            .map(|x| x.into_iter().flatten().collect::<Vec<_>>())?;

    orders.extend(paged_orders);

    Ok(orders)
}

async fn get_market_orders_page(
    client: EsiClient,
    region: usize,
    page: usize,
) -> Result<Vec<models::MarketRegionOrdersItem>, EsiError> {
    client
        .get(&format!("/markets/{}/orders/?page={}", region, page))
        .await
}

pub async fn get_market_region_types(
    client: EsiClient,
    region: usize,
) -> Result<Vec<i32>, EsiError> {
    let response = client
        .get_response(&format!("/markets/{}/types/?page=1", region))
        .await?;
    let pages = extract_pages(&response)?;

    let mut types = response
        .json::<models::MarketRegionTypes>()
        .await
        .map_err(|e| EsiError::MarketRegionType(e, region, 1))?;

    let paged_types = try_join_all(
        (2..pages).map(|page| get_market_region_types_page(client.clone(), region, page)),
    )
    .await
    .map(|x| x.into_iter().flatten().collect::<Vec<_>>())?;

    types.extend(paged_types);

    types.sort();

    Ok(types)
}

async fn get_market_region_types_page(
    client: EsiClient,
    region: usize,
    page: usize,
) -> Result<Vec<i32>, EsiError> {
    client
        .get(&format!(
            "/markets/{}/types/?datasource=tranquility&page={}",
            region, page
        ))
        .await
}

pub async fn is_published(client: EsiClient, type_id: usize) -> Result<bool, EsiError> {
    let path = format!("/universe/types/{}/", type_id);
    client
        .get::<UniverseTypeId>(&path)
        .await
        .map(|s| s.published)
}

fn extract_pages(response: &reqwest::Response) -> Result<usize, EsiError> {
    let pages = response
        .headers()
        .get("x-pages")
        .ok_or(EsiError::NoPages)?
        .to_str()
        .unwrap()
        .parse::<usize>()
        .unwrap();
    Ok(pages)
}
