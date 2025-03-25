use crate::entity::WatchlistItem;
use crate::interactor::db;
use crate::solana::jupiter::price_service::PriceService;
use crate::solana::jupiter::token_repository::TokenRepository;
use crate::validate_solana_address;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;

#[async_trait]
pub trait WatchlistInteractor: Send + Sync {
    async fn get_watchlist(&self, telegram_id: i64) -> Result<Vec<WatchlistItem>>;
    async fn add_to_watchlist(
        &self,
        telegram_id: i64,
        token_address: &str,
    ) -> Result<WatchlistItem>;
    async fn remove_from_watchlist(&self, telegram_id: i64, token_address: &str) -> Result<bool>;
    async fn get_watchlist_item(
        &self,
        telegram_id: i64,
        token_address: &str,
    ) -> Result<Option<WatchlistItem>>;
    async fn validate_token_address(&self, token_address: &str) -> Result<bool>;
    async fn refresh_watchlist_prices(&self, telegram_id: i64) -> Result<Vec<WatchlistItem>>;
}

pub struct WatchlistInteractorImpl {
    db_pool: Arc<PgPool>,
    price_service: Arc<dyn PriceService + Send + Sync>,
    token_repository: Arc<dyn TokenRepository + Send + Sync>,
}

impl WatchlistInteractorImpl {
    pub fn new(
        db_pool: Arc<PgPool>,
        price_service: Arc<dyn PriceService + Send + Sync>,
        token_repository: Arc<dyn TokenRepository + Send + Sync>,
    ) -> Self {
        Self {
            db_pool,
            price_service,
            token_repository,
        }
    }
}

#[async_trait]
impl WatchlistInteractor for WatchlistInteractorImpl {
    async fn get_watchlist(&self, telegram_id: i64) -> Result<Vec<WatchlistItem>> {
        db::get_user_watchlist(&self.db_pool, telegram_id)
            .await
            .map_err(|e| anyhow!("Failed to get watchlist: {}", e))
    }

    async fn add_to_watchlist(
        &self,
        telegram_id: i64,
        token_address: &str,
    ) -> Result<WatchlistItem> {
        // First validate token address
        if !self.validate_token_address(token_address).await? {
            return Err(anyhow!("Invalid token address"));
        }

        // Get token information
        let token = self.token_repository.get_token_by_id(token_address).await?;

        // Get current token price
        let price_info = self.price_service.get_token_price(token_address).await?;

        // Add to watchlist
        let id = db::add_to_watchlist(
            &self.db_pool,
            telegram_id,
            token_address,
            &token.symbol,
            price_info.price_in_sol,
        )
        .await
        .map_err(|e| anyhow!("Failed to add to watchlist: {}", e))?;

        // Fetch the newly created item
        let item = db::get_watchlist_item(&self.db_pool, telegram_id, token_address)
            .await
            .map_err(|e| anyhow!("Failed to get watchlist item: {}", e))?
            .ok_or_else(|| anyhow!("Failed to find watchlist item after adding"))?;

        Ok(item)
    }

    async fn remove_from_watchlist(&self, telegram_id: i64, token_address: &str) -> Result<bool> {
        db::remove_from_watchlist(&self.db_pool, telegram_id, token_address)
            .await
            .map_err(|e| anyhow!("Failed to remove from watchlist: {}", e))
    }

    async fn get_watchlist_item(
        &self,
        telegram_id: i64,
        token_address: &str,
    ) -> Result<Option<WatchlistItem>> {
        db::get_watchlist_item(&self.db_pool, telegram_id, token_address)
            .await
            .map_err(|e| anyhow!("Failed to get watchlist item: {}", e))
    }

    async fn validate_token_address(&self, token_address: &str) -> Result<bool> {
        // First check if it's a valid Solana address
        if !validate_solana_address(token_address) {
            return Ok(false);
        }

        // Then check if it's actually a token mint address
        match self.token_repository.get_token_by_id(token_address).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn refresh_watchlist_prices(&self, telegram_id: i64) -> Result<Vec<WatchlistItem>> {
        // Get current watchlist
        let watchlist = self.get_watchlist(telegram_id).await?;

        // For each token, get current price and update it
        for item in &watchlist {
            // Get current price
            if let Ok(price_info) = self
                .price_service
                .get_token_price(&item.token_address)
                .await
            {
                // Update price in database
                let _ = db::update_watchlist_price(
                    &self.db_pool,
                    telegram_id,
                    &item.token_address,
                    price_info.price_in_sol,
                )
                .await;
            }
        }

        // Get updated watchlist
        self.get_watchlist(telegram_id).await
    }
}
