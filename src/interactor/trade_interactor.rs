use crate::entity::{BotError, OrderType, Token};
use crate::interactor::db;
use crate::solana::jupiter::quote_service::QuoteService;
use crate::solana::jupiter::swap_service::SwapService;
use crate::solana::jupiter::token_repository::JupiterTokenRepository;
use crate::solana::jupiter::token_repository::TokenRepository;
use crate::solana::jupiter::PriceService;
use crate::{solana, validate_solana_address};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use sqlx::PgPool;
use std::str::FromStr;
use std::sync::Arc;

pub struct TradeResult {
    pub token_address: String,
    pub token_symbol: String,
    pub amount: f64,
    pub price_in_sol: f64,
    pub total_sol: f64,
    pub signature: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
}

#[async_trait]
pub trait TradeInteractor: Send + Sync {
    async fn validate_token_address(&self, token_address: &str) -> Result<bool>;
    async fn get_token_info(&self, token_address: &str) -> Result<(String, f64, f64)>;
    async fn validate_buy_amount(&self, amount_text: &str) -> Result<f64>;
    async fn validate_sell_amount(
        &self,
        amount_text: &str,
        token_address: &str,
        user_address: &str,
    ) -> Result<f64>;
    async fn execute_trade(
        &self,
        telegram_id: i64,
        trade_type: &OrderType,
        token_address: &str,
        token_symbol: &str,
        amount: f64,
        price_in_sol: f64,
    ) -> Result<TradeResult>;
}

pub struct TradeInteractorImpl<T, Q>
where
    T: TokenRepository,
    Q: QuoteService,
{
    db_pool: Arc<PgPool>,
    solana_client: Arc<RpcClient>,
    price_service: Arc<dyn PriceService + Send + Sync>,
    token_repository: Arc<dyn TokenRepository + Send + Sync>,
    swap_service: Arc<SwapService<T, Q>>,
}

impl<T, Q> TradeInteractorImpl<T, Q>
where
    T: TokenRepository + 'static,
    Q: QuoteService + 'static,
{
    pub fn new(
        db_pool: Arc<PgPool>,
        solana_client: Arc<RpcClient>,
        price_service: Arc<dyn PriceService + Send + Sync>,
        token_repository: Arc<dyn TokenRepository + Send + Sync>,
        swap_service: Arc<SwapService<T, Q>>,
    ) -> Self {
        Self {
            db_pool,
            solana_client,
            price_service,
            token_repository,
            swap_service,
        }
    }

    async fn get_token_by_address(&self, token_address: &str) -> Result<Token> {
        self.token_repository.get_token_by_id(token_address).await
    }

    pub async fn get_token_balance(&self, token_address: &str, user_address: &str) -> Result<f64> {
        let token_balances = solana::get_token_balances(&self.solana_client, user_address).await?;

        let token_balance = token_balances
            .iter()
            .find(|balance| balance.mint_address == token_address)
            .map(|balance| balance.amount)
            .unwrap_or(0.0);

        Ok(token_balance)
    }

    // Helper method to convert token amount to proper units
    async fn convert_token_amount_for_swap(&self, amount: f64, token_address: &str) -> Result<f64> {
        let token = self.get_token_by_address(token_address).await?;

        // For display we use the token amount as is, but for swap we need to consider decimals
        // This is handled internally by the swap service, so we don't need to adjust here
        Ok(amount)
    }
}

#[async_trait]
impl<T, Q> TradeInteractor for TradeInteractorImpl<T, Q>
where
    T: TokenRepository + Send + Sync + 'static,
    Q: QuoteService + Send + Sync + 'static,
{
    async fn validate_token_address(&self, token_address: &str) -> Result<bool> {
        // First check if it's a valid Solana address
        if !validate_solana_address(token_address) {
            return Ok(false);
        }

        // Then check if it's actually a token mint address
        match self.get_token_by_address(token_address).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn get_token_info(&self, token_address: &str) -> Result<(String, f64, f64)> {
        // Get token information to display to the user
        let token = self.get_token_by_address(token_address).await?;

        // Get token price info
        let price_info = self.price_service.get_token_price(token_address).await?;

        Ok((
            token.symbol,
            price_info.price_in_sol,
            price_info.price_in_usdc,
        ))
    }

    async fn validate_buy_amount(&self, amount_text: &str) -> Result<f64> {
        match amount_text.parse::<f64>() {
            Ok(amount) if amount > 0.0 => Ok(amount),
            Ok(_) => Err(anyhow!("Amount must be greater than zero")),
            Err(_) => Err(anyhow!("Invalid amount format. Please enter a number.")),
        }
    }

    async fn validate_sell_amount(
        &self,
        amount_text: &str,
        token_address: &str,
        user_address: &str,
    ) -> Result<f64> {
        // Check if user wants to sell all tokens
        if amount_text.to_lowercase() == "all" {
            // Get the user's token balance
            let token_balance = self.get_token_balance(token_address, user_address).await?;

            if token_balance <= 0.0 {
                return Err(anyhow!("You don't have any tokens to sell"));
            }

            return Ok(token_balance);
        }

        // Otherwise, validate as a normal number
        match amount_text.parse::<f64>() {
            Ok(amount) if amount > 0.0 => {
                // Verify user has enough tokens
                let token_balance = self.get_token_balance(token_address, user_address).await?;

                if amount > token_balance {
                    return Err(anyhow!(
                        "Insufficient balance. You only have {} tokens",
                        token_balance
                    ));
                }

                Ok(amount)
            }
            Ok(_) => Err(anyhow!("Amount must be greater than zero")),
            Err(_) => Err(anyhow!(
                "Invalid amount format. Please enter a number or 'All'"
            )),
        }
    }
    async fn execute_trade(
        &self,
        telegram_id: i64,
        trade_type: &OrderType,
        token_address: &str,
        token_symbol: &str,
        amount: f64,
        price_in_sol: f64,
    ) -> Result<TradeResult> {
        // Get user wallet info
        let user = db::get_user_by_telegram_id(&self.db_pool, telegram_id).await?;

        match (user.solana_address, user.encrypted_private_key) {
            (Some(user_address), Some(keypair_base58)) => {
                // Get user's keypair
                let keypair = match solana::keypair_from_base58(&keypair_base58) {
                    Ok(k) => k,
                    Err(e) => {
                        return Ok(TradeResult {
                            token_address: token_address.to_string(),
                            token_symbol: token_symbol.to_string(),
                            amount,
                            price_in_sol,
                            total_sol: amount * price_in_sol,
                            signature: None,
                            success: false,
                            error_message: Some(format!("Error with private key: {}", e)),
                        });
                    }
                };

                // Total SOL for the trade
                let total_sol = amount * price_in_sol;

                // Execute the trade based on trade type
                if trade_type == &OrderType::Buy {
                    self.execute_buy_trade(
                        telegram_id,
                        &keypair,
                        &user_address,
                        token_address,
                        token_symbol,
                        amount,
                        price_in_sol,
                        total_sol,
                    )
                    .await
                } else {
                    // SELL
                    self.execute_sell_trade(
                        telegram_id,
                        &keypair,
                        &user_address,
                        token_address,
                        token_symbol,
                        amount,
                        price_in_sol,
                        total_sol,
                    )
                    .await
                }
            }
            _ => Ok(TradeResult {
                token_address: token_address.to_string(),
                token_symbol: token_symbol.to_string(),
                amount,
                price_in_sol,
                total_sol: amount * price_in_sol,
                signature: None,
                success: false,
                error_message: Some(
                    "Wallet not found. Use /create_wallet to create a new wallet.".to_string(),
                ),
            }),
        }
    }
}

// Implementation of private helper methods
impl<T, Q> TradeInteractorImpl<T, Q>
where
    T: TokenRepository + Send + Sync + 'static,
    Q: QuoteService + Send + Sync + 'static,
{
    async fn execute_buy_trade(
        &self,
        telegram_id: i64,
        keypair: &Keypair,
        user_address: &str,
        token_address: &str,
        token_symbol: &str,
        amount: f64,
        price_in_sol: f64,
        total_sol: f64,
    ) -> Result<TradeResult> {
        // For BUY: We're trading from SOL (wrapped SOL) to the target token
        let source_token = "So11111111111111111111111111111111111111112"; // Wrapped SOL address
        let target_token = token_address;

        // Check if user has enough SOL
        let user_pubkey = keypair.pubkey();
        let sol_balance =
            solana::get_sol_balance(&self.solana_client, &user_pubkey.to_string()).await?;

        if sol_balance < total_sol {
            return Ok(TradeResult {
                token_address: token_address.to_string(),
                token_symbol: token_symbol.to_string(),
                amount,
                price_in_sol,
                total_sol,
                signature: None,
                success: false,
                error_message: Some(format!(
                    "Insufficient SOL balance. Required: {} SOL",
                    total_sol
                )),
            });
        }

        // Calculate how much SOL we need to send
        let sol_amount = amount * price_in_sol;

        // For slippage, use a default value
        let slippage = 0.01; // 1%

        // Prepare the swap
        let swap_response = match self
            .swap_service
            .prepare_swap(
                sol_amount,
                source_token,
                target_token,
                slippage,
                user_address,
            )
            .await
        {
            Ok(response) => response,
            Err(e) => {
                return Ok(TradeResult {
                    token_address: token_address.to_string(),
                    token_symbol: token_symbol.to_string(),
                    amount,
                    price_in_sol,
                    total_sol,
                    signature: None,
                    success: false,
                    error_message: Some(format!("Failed to prepare swap: {}", e)),
                });
            }
        };

        // Execute the swap transaction
        match self
            .swap_service
            .execute_swap_transaction(&self.solana_client, keypair, &swap_response)
            .await
        {
            Ok(signature) => {
                // Record the trade in the database
                let _ = db::record_trade(
                    &self.db_pool,
                    telegram_id,
                    token_address,
                    token_symbol,
                    amount,
                    price_in_sol,
                    total_sol,
                    "BUY",
                    &Some(signature.clone()),
                    "SUCCESS",
                )
                .await;

                Ok(TradeResult {
                    token_address: token_address.to_string(),
                    token_symbol: token_symbol.to_string(),
                    amount,
                    price_in_sol,
                    total_sol,
                    signature: Some(signature),
                    success: true,
                    error_message: None,
                })
            }
            Err(e) => {
                // Record failed transaction
                let _ = db::record_trade(
                    &self.db_pool,
                    telegram_id,
                    token_address,
                    token_symbol,
                    amount,
                    price_in_sol,
                    total_sol,
                    "BUY",
                    &None::<String>,
                    "FAILED",
                )
                .await;

                Ok(TradeResult {
                    token_address: token_address.to_string(),
                    token_symbol: token_symbol.to_string(),
                    amount,
                    price_in_sol,
                    total_sol,
                    signature: None,
                    success: false,
                    error_message: Some(format!("Failed to execute swap: {}", e)),
                })
            }
        }
    }

    async fn execute_sell_trade(
        &self,
        telegram_id: i64,
        keypair: &Keypair,
        user_address: &str,
        token_address: &str,
        token_symbol: &str,
        amount: f64,
        price_in_sol: f64,
        total_sol: f64,
    ) -> Result<TradeResult> {
        // For SELL: We're trading from the token to SOL (wrapped SOL)
        let source_token = token_address;
        let target_token = "So11111111111111111111111111111111111111112"; // Wrapped SOL address

        // Check if user has enough tokens to sell
        let token_balances = solana::get_token_balances(&self.solana_client, &user_address).await?;
        let token_balance = token_balances
            .iter()
            .find(|balance| balance.mint_address == token_address)
            .map(|balance| balance.amount)
            .unwrap_or(0.0);

        if token_balance < amount {
            return Ok(TradeResult {
                token_address: token_address.to_string(),
                token_symbol: token_symbol.to_string(),
                amount,
                price_in_sol,
                total_sol,
                signature: None,
                success: false,
                error_message: Some(format!(
                    "Insufficient token balance. Required: {} {}",
                    amount, token_symbol
                )),
            });
        }

        // For slippage, use a default value
        let slippage = 0.01; // 1%

        // Prepare the swap
        let swap_response = match self
            .swap_service
            .prepare_swap(amount, source_token, target_token, slippage, user_address)
            .await
        {
            Ok(response) => response,
            Err(e) => {
                return Ok(TradeResult {
                    token_address: token_address.to_string(),
                    token_symbol: token_symbol.to_string(),
                    amount,
                    price_in_sol,
                    total_sol,
                    signature: None,
                    success: false,
                    error_message: Some(format!("Failed to prepare swap: {}", e)),
                });
            }
        };

        // Execute the swap transaction
        match self
            .swap_service
            .execute_swap_transaction(&self.solana_client, keypair, &swap_response)
            .await
        {
            Ok(signature) => {
                // Record the trade in the database
                let _ = db::record_trade(
                    &self.db_pool,
                    telegram_id,
                    token_address,
                    token_symbol,
                    amount,
                    price_in_sol,
                    total_sol,
                    "SELL",
                    &Some(signature.clone()),
                    "SUCCESS",
                )
                .await;

                Ok(TradeResult {
                    token_address: token_address.to_string(),
                    token_symbol: token_symbol.to_string(),
                    amount,
                    price_in_sol,
                    total_sol,
                    signature: Some(signature),
                    success: true,
                    error_message: None,
                })
            }
            Err(e) => {
                // Record failed transaction
                let _ = db::record_trade(
                    &self.db_pool,
                    telegram_id,
                    token_address,
                    token_symbol,
                    amount,
                    price_in_sol,
                    total_sol,
                    "SELL",
                    &None::<String>,
                    "FAILED",
                )
                .await;

                Ok(TradeResult {
                    token_address: token_address.to_string(),
                    token_symbol: token_symbol.to_string(),
                    amount,
                    price_in_sol,
                    total_sol,
                    signature: None,
                    success: false,
                    error_message: Some(format!("Failed to execute swap: {}", e)),
                })
            }
        }
    }
}
