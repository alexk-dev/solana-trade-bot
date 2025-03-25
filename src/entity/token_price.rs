use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPrice {
    pub token_id: String,   // Token ID (mint)
    pub symbol: String,     // Token symbol
    pub price_in_sol: f64,  // Price in SOL
    pub price_in_usdc: f64, // Price in USDC
    pub timestamp: u64,     // Timestamp of price retrieval
}
