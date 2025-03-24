use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The type of limit order
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LimitOrderType {
    Buy,
    Sell,
}

impl std::fmt::Display for LimitOrderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LimitOrderType::Buy => write!(f, "BUY"),
            LimitOrderType::Sell => write!(f, "SELL"),
        }
    }
}

/// Status of the limit order
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LimitOrderStatus {
    Active,
    Filled,
    Cancelled,
    Failed,
}

impl std::fmt::Display for LimitOrderStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LimitOrderStatus::Active => write!(f, "ACTIVE"),
            LimitOrderStatus::Filled => write!(f, "FILLED"),
            LimitOrderStatus::Cancelled => write!(f, "CANCELLED"),
            LimitOrderStatus::Failed => write!(f, "FAILED"),
        }
    }
}

/// Limit order entity
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LimitOrder {
    pub id: i32,
    pub user_id: i32,
    pub token_address: String,
    pub token_symbol: String,
    pub order_type: String, // "BUY" or "SELL"
    pub price_in_sol: f64,
    pub amount: f64,
    pub current_price_in_sol: Option<f64>,
    pub tx_signature: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: String,
}

/// State for the limit order dialogue
#[derive(Debug, Clone)]
pub enum LimitOrderState {
    AwaitingOrderType,
    AwaitingTokenAddress {
        order_type: LimitOrderType,
    },
    AwaitingPriceAndAmount {
        order_type: LimitOrderType,
        token_address: String,
        token_symbol: String,
        current_price_in_sol: f64,
        current_price_in_usdc: f64,
    },
    AwaitingConfirmation {
        order_type: LimitOrderType,
        token_address: String,
        token_symbol: String,
        price_in_sol: f64,
        amount: f64,
        total_sol: f64,
    },
}
