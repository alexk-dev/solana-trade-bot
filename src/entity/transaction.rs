use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Transaction {
    pub id: i32,
    pub user_id: i32,
    pub recipient_address: String,
    pub amount: f64,
    pub token_symbol: String,
    pub tx_signature: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub status: String,
}
