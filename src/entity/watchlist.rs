use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WatchlistItem {
    pub id: i32,
    pub user_id: i32,
    pub token_address: String,
    pub token_symbol: String,
    pub last_price_in_sol: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl WatchlistItem {
    // Format price for display
    pub fn format_price(&self) -> String {
        format!("{:.6} SOL", self.last_price_in_sol)
    }
}
