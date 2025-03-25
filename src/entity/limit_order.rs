use anyhow::anyhow;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// The type of limit order
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderType {
    Buy,
    Sell,
}

impl std::fmt::Display for OrderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderType::Buy => write!(f, "BUY"),
            OrderType::Sell => write!(f, "SELL"),
        }
    }
}

impl FromStr for OrderType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "BUY" => Ok(OrderType::Buy),
            "SELL" => Ok(OrderType::Sell),
            _ => Err(anyhow!("Invalid order type: {}", s)),
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
    pub amount: f64,    // Token amount
    pub total_sol: f64, // Total SOL volume
    pub current_price_in_sol: Option<f64>,
    pub tx_signature: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: String,
    pub retry_count: i32, // Track retry attempts
}

/// State for the limit order dialogue
#[derive(Debug, Clone)]
pub enum LimitOrderState {
    AwaitingOrderType,
    AwaitingTokenAddress {
        order_type: OrderType,
    },
    AwaitingPriceAndAmount {
        order_type: OrderType,
        token_address: String,
        token_symbol: String,
        current_price_in_sol: f64,
        current_price_in_usdc: f64,
    },
    AwaitingConfirmation {
        order_type: OrderType,
        token_address: String,
        token_symbol: String,
        price_in_sol: f64,
        amount: f64,
        total_sol: f64,
    },
}
