use crate::entity::{BotError, TokenBalance};
use crate::interactor::db;
use crate::solana;
use crate::solana::jupiter::PriceService;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use solana_client::nonblocking::rpc_client::RpcClient;
use sqlx::PgPool;
use std::sync::Arc;

#[async_trait]
pub trait BalanceInteractor: Send + Sync {
    async fn get_wallet_balances(
        &self,
        telegram_id: i64,
    ) -> Result<(String, f64, Vec<TokenBalance>, Vec<(String, f64)>)>;
}

pub struct BalanceInteractorImpl {
    db_pool: Arc<PgPool>,
    solana_client: Arc<RpcClient>,
    price_service: Arc<dyn PriceService + Send + Sync>,
}

impl BalanceInteractorImpl {
    pub fn new(
        db_pool: Arc<PgPool>,
        solana_client: Arc<RpcClient>,
        price_service: Arc<dyn PriceService + Send + Sync>,
    ) -> Self {
        Self {
            db_pool,
            solana_client,
            price_service,
        }
    }
}

#[async_trait]
impl BalanceInteractor for BalanceInteractorImpl {
    async fn get_wallet_balances(
        &self,
        telegram_id: i64,
    ) -> Result<(String, f64, Vec<TokenBalance>, Vec<(String, f64)>)> {
        // Get user's wallet address
        let user = db::get_user_by_telegram_id(&self.db_pool, telegram_id).await?;

        let address = user
            .solana_address
            .ok_or_else(|| BotError::WalletNotFound)?;

        // Get SOL balance
        let sol_balance = solana::get_sol_balance(&self.solana_client, &address).await?;

        // Get token balances
        let token_balances = match solana::get_token_balances(&self.solana_client, &address).await {
            Ok(balances) => balances,
            Err(e) => {
                return Err(anyhow!("Error fetching token balances: {}", e));
            }
        };

        // Initialize vector for USD values
        let mut usd_values = Vec::new();

        if !token_balances.is_empty() {
            // Get SOL price first for reference
            let sol_price = match self.price_service.get_sol_price().await {
                Ok(price) => price,
                Err(e) => {
                    return Err(anyhow!("Error fetching SOL price: {}", e));
                }
            };

            // Calculate SOL USD value
            let sol_usd = sol_balance * sol_price;
            usd_values.push((String::from("SOL"), sol_usd));

            // Get prices for other tokens
            for token in &token_balances {
                if token.amount > 0.0 {
                    match self
                        .price_service
                        .get_token_price(&token.mint_address)
                        .await
                    {
                        Ok(price_info) => {
                            let usd_value = token.amount * price_info.price_in_usdc;
                            usd_values.push((token.symbol.clone(), usd_value));
                        }
                        Err(e) => {
                            usd_values.push((token.symbol.clone(), 0.0)); // Default to 0 if error
                        }
                    }
                }
            }
        }

        Ok((address, sol_balance, token_balances, usd_values))
    }
}
