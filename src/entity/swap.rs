use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Swap {
    pub id: i32,
    pub user_id: i32,
    pub from_token: String,
    pub to_token: String,
    pub amount_in: f64,
    pub amount_out: f64,
    pub tx_signature: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub status: String,
}
