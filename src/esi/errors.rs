#[derive(Debug)]
pub enum EsiError {
    ErrorResponse,
    NoPages,
    MarketRegionType(reqwest::Error, usize, usize),
    MarketOrder(reqwest::Error, usize, usize),
    ErrorLimited,
    NotPublished(usize),
    JsonError(reqwest::Error),
    ConnectionError(reqwest::Error),
}
