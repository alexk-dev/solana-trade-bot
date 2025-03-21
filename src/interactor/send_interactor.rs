use crate::interactor::db;
use crate::solana;
use crate::utils;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use solana_client::nonblocking::rpc_client::RpcClient;
use sqlx::PgPool;
use std::sync::Arc;

pub struct TransactionResult {
    pub recipient: String,
    pub amount: f64,
    pub token: String,
    pub signature: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
}

#[async_trait]
pub trait SendInteractor: Send + Sync {
    async fn validate_address(&self, address: &str) -> Result<bool>;
    async fn parse_amount_and_token(&self, amount_text: &str) -> Result<(f64, String)>;
    async fn send_transaction(
        &self,
        telegram_id: i64,
        recipient: &str,
        amount: f64,
        token: &str,
    ) -> Result<TransactionResult>;
}

pub struct SendInteractorImpl {
    db_pool: Arc<PgPool>,
    solana_client: Arc<RpcClient>,
}

impl SendInteractorImpl {
    pub fn new(db_pool: Arc<PgPool>, solana_client: Arc<RpcClient>) -> Self {
        Self {
            db_pool,
            solana_client,
        }
    }
}

#[async_trait]
impl SendInteractor for SendInteractorImpl {
    async fn validate_address(&self, address: &str) -> Result<bool> {
        Ok(utils::validate_solana_address(address))
    }

    async fn parse_amount_and_token(&self, amount_text: &str) -> Result<(f64, String)> {
        match utils::parse_amount_and_token(amount_text) {
            Some((amount, token)) => Ok((amount, token.to_string())),
            None => Err(anyhow!(
                "Invalid amount format. Please enter in the format '0.5 SOL' or '100 USDC'"
            )),
        }
    }

    async fn send_transaction(
        &self,
        telegram_id: i64,
        recipient: &str,
        amount: f64,
        token: &str,
    ) -> Result<TransactionResult> {
        // Get user wallet info
        let user = db::get_user_by_telegram_id(&self.db_pool, telegram_id).await?;

        match (user.solana_address, user.encrypted_private_key) {
            (Some(sender_address), Some(keypair_base58)) => {
                // Get private key
                let keypair = match solana::keypair_from_base58(&keypair_base58) {
                    Ok(k) => k,
                    Err(e) => {
                        return Ok(TransactionResult {
                            recipient: recipient.to_string(),
                            amount,
                            token: token.to_string(),
                            signature: None,
                            success: false,
                            error_message: Some(format!("Error with private key: {}", e)),
                        });
                    }
                };

                // Send transaction
                let result = if token.to_uppercase() == "SOL" {
                    solana::send_sol(&self.solana_client, &keypair, recipient, amount).await
                } else {
                    solana::send_spl_token(&self.solana_client, &keypair, recipient, token, amount)
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
                            token,
                            &Some(signature.clone()),
                            "SUCCESS",
                        )
                        .await;

                        Ok(TransactionResult {
                            recipient: recipient.to_string(),
                            amount,
                            token: token.to_string(),
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
                            token,
                            &None::<String>,
                            "FAILED",
                        )
                        .await;

                        Ok(TransactionResult {
                            recipient: recipient.to_string(),
                            amount,
                            token: token.to_string(),
                            signature: None,
                            success: false,
                            error_message: Some(e.to_string()),
                        })
                    }
                }
            }
            _ => Ok(TransactionResult {
                recipient: recipient.to_string(),
                amount,
                token: token.to_string(),
                signature: None,
                success: false,
                error_message: Some(
                    "Wallet not found. Use /create_wallet to create a new wallet.".to_string(),
                ),
            }),
        }
    }
}
