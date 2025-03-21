use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPrice {
    pub token_id: String,   // ID токена (mint)
    pub symbol: String,     // Символ токена
    pub price_in_sol: f64,  // Цена в SOL
    pub price_in_usdc: f64, // Цена в USDC
    pub timestamp: u64,     // Время получения цены
}
