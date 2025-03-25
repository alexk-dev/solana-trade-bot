use anyhow::{anyhow, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;

use crate::entity::User;
use crate::interactor::db;

#[async_trait]
pub trait SettingsInteractor: Send + Sync {
    async fn get_user_settings(&self, telegram_id: i64) -> Result<User>;
    async fn update_slippage(&self, telegram_id: i64, slippage: f64) -> Result<f64>;
}

pub struct SettingsInteractorImpl {
    db_pool: Arc<PgPool>,
}

impl SettingsInteractorImpl {
    pub fn new(db_pool: Arc<PgPool>) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl SettingsInteractor for SettingsInteractorImpl {
    async fn get_user_settings(&self, telegram_id: i64) -> Result<User> {
        db::get_user_by_telegram_id(&self.db_pool, telegram_id)
            .await
            .map_err(|e| anyhow!("Failed to get user settings: {}", e))
    }

    async fn update_slippage(&self, telegram_id: i64, slippage: f64) -> Result<f64> {
        // Limit slippage to reasonable range (0.1% to 5%)
        let slippage = slippage.max(0.1).min(5.0);

        db::update_user_slippage(&self.db_pool, telegram_id, slippage)
            .await
            .map_err(|e| anyhow!("Failed to update slippage setting: {}", e))?;

        Ok(slippage)
    }
}
