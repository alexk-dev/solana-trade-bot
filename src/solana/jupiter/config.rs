/// Application configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// URL for quote API
    pub quote_api_url: String,

    /// URL for price API
    pub price_api_url: String,

    /// SOL token address (wrapped)
    pub sol_token_address: String,

    /// USDC token address
    pub usdc_token_address: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            quote_api_url: "https://quote-api.jup.ag/v6".to_string(),
            price_api_url: "https://price.jup.ag/v1".to_string(),
            sol_token_address: "So11111111111111111111111111111111111111112".to_string(),
            usdc_token_address: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
        }
    }
}

impl Config {
    /// Создает новую конфигурацию из переменных окружения
    pub fn from_env() -> Self {
        use std::env;

        Self {
            quote_api_url: env::var("QUOTE_API_URL")
                .unwrap_or_else(|_| "https://quote-api.jup.ag/v6".to_string()),
            price_api_url: env::var("PRICE_API_URL")
                .unwrap_or_else(|_| "https://price.jup.ag/v1".to_string()),
            sol_token_address: env::var("SOL_TOKEN_ADDRESS")
                .unwrap_or_else(|_| "So11111111111111111111111111111111111111112".to_string()),
            usdc_token_address: env::var("USDC_TOKEN_ADDRESS")
                .unwrap_or_else(|_| "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string()),
        }
    }
}
