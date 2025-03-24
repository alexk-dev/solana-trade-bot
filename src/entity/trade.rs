use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Trade {
    pub id: i32,
    pub user_id: i32,
    pub token_address: String,
    pub token_symbol: String,
    pub amount: f64,
    pub price_in_sol: f64,
    pub price_in_usdc: f64,
    pub total_paid: f64,
    pub trade_type: String, // "BUY" or "SELL"
    pub tx_signature: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub status: String,
}

#[derive(Debug, Clone)]
pub enum TradeState {
    AwaitingTokenAddress {
        trade_type: String,
    },
    AwaitingAmount {
        trade_type: String,
        token_address: String,
        token_symbol: String,
        price_in_sol: f64,
        price_in_usdc: f64,
    },
    AwaitingConfirmation {
        trade_type: String,
        token_address: String,
        token_symbol: String,
        amount: f64,
        price_in_sol: f64,
        total_sol: f64,
    },
}
