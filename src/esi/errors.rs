
#[derive(Debug)]
pub enum EsiError {
    ErrorResponse,
    NoPages,
    MarketHistory(reqwest::Error, usize, usize),
    MarketRegionType(reqwest::Error, usize, usize),
    MarketOrder(reqwest::Error, usize, usize),
    PublishCheck(reqwest::Error, usize),
    ErrorLimited,
    NotPublished(usize),
}
