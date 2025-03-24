use crate::entity::{LimitOrder, LimitOrderType};
use crate::interactor::db;
use crate::solana::jupiter::price_service::PriceService;
use crate::solana::jupiter::token_repository::TokenRepository;
use crate::validate_solana_address;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use solana_client::nonblocking::rpc_client::RpcClient;
use sqlx::PgPool;
use std::sync::Arc;

pub struct LimitOrderResult {
    pub token_address: String,
    pub token_symbol: String,
    pub order_type: LimitOrderType,
    pub price_in_sol: f64,
    pub amount: f64,
    pub total_sol: f64,
    pub order_id: Option<i32>,
    pub success: bool,
    pub error_message: Option<String>,
}

#[async_trait]
pub trait LimitOrderInteractor: Send + Sync {
    async fn validate_token_address(&self, token_address: &str) -> Result<bool>;
    async fn get_token_info(&self, token_address: &str) -> Result<(String, f64, f64)>;

    async fn validate_order_price_and_amount(
        &self,
        price_amount_text: &str,
        order_type: &LimitOrderType,
        token_address: &str,
        token_symbol: &str,
        telegram_id: i64,
    ) -> Result<(f64, f64, f64)>;

    async fn create_limit_order(
        &self,
        telegram_id: i64,
        order_type: &LimitOrderType,
        token_address: &str,
        token_symbol: &str,
        price_in_sol: f64,
        amount: f64,
    ) -> Result<LimitOrderResult>;

    async fn get_active_limit_orders(&self, telegram_id: i64) -> Result<Vec<LimitOrder>>;

    async fn cancel_limit_order(&self, order_id: i32) -> Result<bool>;
}

pub struct LimitOrderInteractorImpl {
    db_pool: Arc<PgPool>,
    solana_client: Arc<RpcClient>,
    price_service: Arc<dyn PriceService + Send + Sync>,
    token_repository: Arc<dyn TokenRepository + Send + Sync>,
}

impl LimitOrderInteractorImpl {
    pub fn new(
        db_pool: Arc<PgPool>,
        solana_client: Arc<RpcClient>,
        price_service: Arc<dyn PriceService + Send + Sync>,
        token_repository: Arc<dyn TokenRepository + Send + Sync>,
    ) -> Self {
        Self {
            db_pool,
            solana_client,
            price_service,
            token_repository,
        }
    }
}

#[async_trait]
impl LimitOrderInteractor for LimitOrderInteractorImpl {
    async fn validate_token_address(&self, token_address: &str) -> Result<bool> {
        // First check if it's a valid Solana address
        if !validate_solana_address(token_address) {
            return Ok(false);
        }

        // Then check if it's actually a token mint address by trying to get its info
        match self.token_repository.get_token_by_id(token_address).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn get_token_info(&self, token_address: &str) -> Result<(String, f64, f64)> {
        // Get token information
        let token = self.token_repository.get_token_by_id(token_address).await?;

        // Get token price info
        let price_info = self.price_service.get_token_price(token_address).await?;

        Ok((
            token.symbol,
            price_info.price_in_sol,
            price_info.price_in_usdc,
        ))
    }

    async fn validate_order_price_and_amount(
        &self,
        price_amount_text: &str,
        order_type: &LimitOrderType,
        token_address: &str,
        token_symbol: &str,
        telegram_id: i64,
    ) -> Result<(f64, f64, f64)> {
        // Expected format: "price amount" - e.g. "0.5 10" for 10 tokens at 0.5 SOL each
        let parts: Vec<&str> = price_amount_text.trim().split_whitespace().collect();

        if parts.len() != 2 {
            return Err(anyhow!(
                "Invalid format. Please enter price and amount separated by space (e.g. '0.5 10')"
            ));
        }

        // Parse price
        let price = match parts[0].parse::<f64>() {
            Ok(p) if p > 0.0 => p,
            Ok(_) => return Err(anyhow!("Price must be greater than zero")),
            Err(_) => return Err(anyhow!("Invalid price format. Please enter a number.")),
        };

        // Parse amount
        let amount = match parts[1].parse::<f64>() {
            Ok(a) if a > 0.0 => a,
            Ok(_) => return Err(anyhow!("Amount must be greater than zero")),
            Err(_) => return Err(anyhow!("Invalid amount format. Please enter a number.")),
        };

        // Calculate total SOL value
        let total_sol = price * amount;

        // For sell orders, verify user has enough tokens
        if *order_type == LimitOrderType::Sell {
            // Get user wallet
            let user = db::get_user_by_telegram_id(&self.db_pool, telegram_id).await?;

            if let Some(user_address) = user.solana_address {
                // Get token balances
                let token_balances =
                    crate::solana::get_token_balances(&self.solana_client, &user_address).await?;

                // Check if user has the token and sufficient balance
                let token_balance = token_balances
                    .iter()
                    .find(|balance| balance.mint_address == token_address)
                    .map(|balance| balance.amount)
                    .unwrap_or(0.0);

                if token_balance < amount {
                    return Err(anyhow!(
                        "Insufficient balance. You only have {} {} tokens",
                        token_balance,
                        token_symbol
                    ));
                }
            } else {
                return Err(anyhow!("Wallet not found. Please create a wallet first."));
            }
        }

        Ok((price, amount, total_sol))
    }

    async fn create_limit_order(
        &self,
        telegram_id: i64,
        order_type: &LimitOrderType,
        token_address: &str,
        token_symbol: &str,
        price_in_sol: f64,
        amount: f64,
    ) -> Result<LimitOrderResult> {
        // Get current price for comparison
        let price_info = self.price_service.get_token_price(token_address).await?;
        let current_price = price_info.price_in_sol;

        // Create the order
        match db::create_limit_order(
            &self.db_pool,
            telegram_id,
            token_address,
            token_symbol,
            order_type,
            price_in_sol,
            amount,
            Some(current_price),
        )
        .await
        {
            Ok(order_id) => Ok(LimitOrderResult {
                token_address: token_address.to_string(),
                token_symbol: token_symbol.to_string(),
                order_type: order_type.clone(),
                price_in_sol,
                amount,
                total_sol: price_in_sol * amount,
                order_id: Some(order_id),
                success: true,
                error_message: None,
            }),
            Err(e) => Ok(LimitOrderResult {
                token_address: token_address.to_string(),
                token_symbol: token_symbol.to_string(),
                order_type: order_type.clone(),
                price_in_sol,
                amount,
                total_sol: price_in_sol * amount,
                order_id: None,
                success: false,
                error_message: Some(format!("Failed to create limit order: {}", e)),
            }),
        }
    }

    async fn get_active_limit_orders(&self, telegram_id: i64) -> Result<Vec<LimitOrder>> {
        db::get_active_limit_orders(&self.db_pool, telegram_id)
            .await
            .map_err(|e| anyhow!("Error fetching limit orders: {}", e))
    }

    async fn cancel_limit_order(&self, order_id: i32) -> Result<bool> {
        match db::cancel_limit_order(&self.db_pool, order_id).await {
            Ok(_) => Ok(true),
            Err(e) => Err(anyhow!("Failed to cancel limit order: {}", e)),
        }
    }
}
