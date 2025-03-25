use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

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
    #[serde(default)]
    pub settings: Option<JsonValue>,
}

// Default user settings
pub fn default_user_settings() -> JsonValue {
    serde_json::json!({
        "slippage": 0.5
    })
}

// Helper methods for User
impl User {
    // Get slippage value from settings (with default fallback)
    pub fn get_slippage(&self) -> f64 {
        match &self.settings {
            Some(settings) => settings
                .get("slippage")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5),
            None => 0.5,
        }
    }

    // Update slippage value in settings
    pub fn with_slippage(mut self, slippage: f64) -> Self {
        // Limit slippage to reasonable range (0.1% to 5%)
        let slippage = slippage.max(0.1).min(5.0);

        // Get current settings or create new default settings
        let mut current_settings = match &self.settings {
            Some(settings) => settings.clone(),
            None => default_user_settings(),
        };

        // Update slippage value
        if let Some(obj) = current_settings.as_object_mut() {
            obj.insert("slippage".to_string(), serde_json::json!(slippage));
        }

        // Set updated settings
        self.settings = Some(current_settings);

        self
    }
}
