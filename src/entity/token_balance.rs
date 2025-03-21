use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    pub symbol: String,
    pub amount: f64,
    pub mint_address: String,
}
