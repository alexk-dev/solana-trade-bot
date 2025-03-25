use crate::entity::{BotError, TokenBalance};
use crate::interactor::db;
use crate::solana;
use crate::solana::jupiter::PriceService;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use solana_client::nonblocking::rpc_client::RpcClient;
use sqlx::PgPool;
use std::sync::Arc;

pub struct WithdrawResult {
    pub token_address: String,
    pub token_symbol: String,
    pub amount: f64,
    pub recipient: String,
    pub signature: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
}

#[async_trait]
pub trait WithdrawInteractor: Send + Sync {
    async fn get_user_tokens(&self, telegram_id: i64) -> Result<Vec<TokenBalance>>;
    async fn get_token_price(&self, token_address: &str) -> Result<(f64, f64)>;
    async fn validate_recipient_address(&self, address: &str) -> Result<bool>;
    async fn validate_withdraw_amount(&self, amount_text: &str, token_balance: f64) -> Result<f64>;
    async fn execute_withdraw(
        &self,
        telegram_id: i64,
        token_address: &str,
        token_symbol: &str,
        recipient: &str,
        amount: f64,
        price_in_sol: f64,
    ) -> Result<WithdrawResult>;
}

pub struct WithdrawInteractorImpl {
    db_pool: Arc<PgPool>,
    solana_client: Arc<RpcClient>,
    price_service: Arc<dyn PriceService + Send + Sync>,
}

impl WithdrawInteractorImpl {
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
impl WithdrawInteractor for WithdrawInteractorImpl {
    async fn get_user_tokens(&self, telegram_id: i64) -> Result<Vec<TokenBalance>> {
        // Get user's wallet address
        let user = db::get_user_by_telegram_id(&self.db_pool, telegram_id).await?;

        let address = user
            .solana_address
            .ok_or_else(|| BotError::WalletNotFound)?;

        // Get token balances
        let token_balances = solana::get_token_balances(&self.solana_client, &address).await?;

        // Get SOL balance
        let sol_balance = solana::get_sol_balance(&self.solana_client, &address).await?;

        // Add SOL as a "token" to the list
        let mut all_balances = token_balances.clone();
        all_balances.push(TokenBalance {
            symbol: "SOL".to_string(),
            amount: sol_balance,
            mint_address: "So11111111111111111111111111111111111111112".to_string(), // Wrapped SOL address
        });

        // Filter out zero balances
        let non_zero_balances = all_balances
            .into_iter()
            .filter(|balance| balance.amount > 0.0)
            .collect();

        Ok(non_zero_balances)
    }

    async fn get_token_price(&self, token_address: &str) -> Result<(f64, f64)> {
        // Get token price in SOL and USDC
        match self.price_service.get_token_price(token_address).await {
            Ok(price_info) => Ok((price_info.price_in_sol, price_info.price_in_usdc)),
            Err(e) => {
                // For SOL, handle special case
                if token_address == "So11111111111111111111111111111111111111112" {
                    // SOL is always 1 SOL, get USDC price
                    let sol_price = self.price_service.get_sol_price().await?;
                    Ok((1.0, sol_price))
                } else {
                    Err(anyhow!("Failed to get token price: {}", e))
                }
            }
        }
    }

    async fn validate_recipient_address(&self, address: &str) -> Result<bool> {
        Ok(crate::utils::validate_solana_address(address))
    }

    async fn validate_withdraw_amount(&self, amount_text: &str, token_balance: f64) -> Result<f64> {
        // Check if user wants to send all tokens
        if amount_text.to_lowercase() == "all" {
            if token_balance <= 0.0 {
                return Err(anyhow!("You don't have any tokens to withdraw"));
            }
            return Ok(token_balance);
        }

        // Check if it's a percentage
        if amount_text.ends_with('%') {
            let percentage_str = amount_text.trim_end_matches('%');
            match percentage_str.parse::<f64>() {
                Ok(percentage) if percentage > 0.0 && percentage <= 100.0 => {
                    let amount = token_balance * (percentage / 100.0);
                    if amount <= 0.0 {
                        return Err(anyhow!("The calculated amount is too small"));
                    }
                    return Ok(amount);
                }
                Ok(_) => return Err(anyhow!("Percentage must be between 0 and 100")),
                Err(_) => {
                    return Err(anyhow!(
                        "Invalid percentage format. Please enter a number followed by %"
                    ))
                }
            }
        }

        // Regular amount validation
        match amount_text.parse::<f64>() {
            Ok(amount) if amount > 0.0 => {
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
                "Invalid amount format. Please enter a number, percentage, or 'All'"
            )),
        }
    }

    async fn execute_withdraw(
        &self,
        telegram_id: i64,
        token_address: &str,
        token_symbol: &str,
        recipient: &str,
        amount: f64,
        price_in_sol: f64,
    ) -> Result<WithdrawResult> {
        // Get user wallet info
        let user = db::get_user_by_telegram_id(&self.db_pool, telegram_id).await?;

        match (user.solana_address, user.encrypted_private_key) {
            (Some(_), Some(keypair_base58)) => {
                // Get private key
                let keypair = match solana::keypair_from_base58(&keypair_base58) {
                    Ok(k) => k,
                    Err(e) => {
                        return Ok(WithdrawResult {
                            token_address: token_address.to_string(),
                            token_symbol: token_symbol.to_string(),
                            amount,
                            recipient: recipient.to_string(),
                            signature: None,
                            success: false,
                            error_message: Some(format!("Error with private key: {}", e)),
                        });
                    }
                };

                // Send transaction
                let result = if token_symbol.to_uppercase() == "SOL" {
                    solana::send_sol(&self.solana_client, &keypair, recipient, amount).await
                } else {
                    solana::send_spl_token(
                        &self.solana_client,
                        &keypair,
                        recipient,
                        token_symbol,
                        amount,
                    )
                    .await
                };

                match result {
                    Ok(signature) => {
                        // Record transaction to database
                        let _ = db::record_transaction(
                            &self.db_pool,
                            telegram_id,
                            recipient,
                            amount,
                            token_symbol,
                            &Some(signature.clone()),
                            "SUCCESS",
                        )
                        .await;

                        Ok(WithdrawResult {
                            token_address: token_address.to_string(),
                            token_symbol: token_symbol.to_string(),
                            amount,
                            recipient: recipient.to_string(),
                            signature: Some(signature),
                            success: true,
                            error_message: None,
                        })
                    }
                    Err(e) => {
                        // Record failed transaction
                        let _ = db::record_transaction(
                            &self.db_pool,
                            telegram_id,
                            recipient,
                            amount,
                            token_symbol,
                            &None::<String>,
                            "FAILED",
                        )
                        .await;

                        Ok(WithdrawResult {
                            token_address: token_address.to_string(),
                            token_symbol: token_symbol.to_string(),
                            amount,
                            recipient: recipient.to_string(),
                            signature: None,
                            success: false,
                            error_message: Some(e.to_string()),
                        })
                    }
                }
            }
            _ => Ok(WithdrawResult {
                token_address: token_address.to_string(),
                token_symbol: token_symbol.to_string(),
                amount,
                recipient: recipient.to_string(),
                signature: None,
                success: false,
                error_message: Some(
                    "Wallet not found. Use /create_wallet to create a new wallet.".to_string(),
                ),
            }),
        }
    }
}
