use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// User model matching the database schema
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: i32,
    pub telegram_id: i64,
    pub username: Option<String>,
    pub solana_address: Option<String>,
    pub encrypted_private_key: Option<String>,
    pub mnemonic: Option<String>,
    pub created_at: DateTime<Utc>,
}
