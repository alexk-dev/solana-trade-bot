use serde::{Deserialize, Serialize};
use sqlx::types::chrono::{DateTime, Utc};

// Swap parameters for the Raydium API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapParams {
    pub amount_in: f64,
    pub source_token: String,
    pub target_token: String,
    pub slippage: f64,
}

// Raydium quote response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumQuote {
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    pub other_amount_threshold: String,
    pub slippage_bps: u32,
    pub route: Vec<String>,
    pub platform_fee: Option<PlatformFee>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformFee {
    pub amount: String,
    pub fee_bps: u32,
}
