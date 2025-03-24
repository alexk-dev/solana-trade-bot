use crate::entity::BotError;
use crate::interactor::db;
use crate::solana;
use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;

#[async_trait]
pub trait WalletInteractor: Send + Sync {
    async fn create_wallet(&self, telegram_id: i64) -> Result<(String, String, String)>;
    async fn get_wallet_info(&self, telegram_id: i64) -> Result<Option<(String, String)>>;
}

pub struct WalletInteractorImpl {
    db_pool: Arc<PgPool>,
}

impl WalletInteractorImpl {
    pub fn new(db_pool: Arc<PgPool>) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl WalletInteractor for WalletInteractorImpl {
    async fn create_wallet(&self, telegram_id: i64) -> Result<(String, String, String)> {
        // Check if user already has a wallet
        let user = db::get_user_by_telegram_id(&self.db_pool, telegram_id).await?;

        if user.solana_address.is_some() {
            return Err(
                BotError::WalletCreationError("User already has a wallet".to_string()).into(),
            );
        }

        // Generate new wallet
        let (mnemonic, keypair, address) = solana::generate_wallet()?;

        // Save wallet info to the database
        db::save_wallet_info(&self.db_pool, telegram_id, &address, &keypair, &mnemonic).await?;

        Ok((mnemonic, keypair, address))
    }

    async fn get_wallet_info(&self, telegram_id: i64) -> Result<Option<(String, String)>> {
        let user = db::get_user_by_telegram_id(&self.db_pool, telegram_id).await?;
        match (user.solana_address, user.mnemonic) {
            (Some(address), Some(mnemonic)) => Ok(Some((address, mnemonic))),
            _ => Ok(None),
        }
    }
}
