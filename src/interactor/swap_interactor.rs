use crate::entity::BotError;
use crate::interactor::db;
use crate::solana;
use crate::solana::jupiter::quote_service::QuoteService;
use crate::solana::jupiter::swap_service::SwapService;
use crate::solana::jupiter::token_repository::TokenRepository;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use solana_client::nonblocking::rpc_client::RpcClient;
use sqlx::PgPool;
use std::sync::Arc;

pub struct SwapResult {
    pub source_token: String,
    pub target_token: String,
    pub amount_in: f64,
    pub amount_out: f64,
    pub signature: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
}

#[async_trait]
pub trait SwapInteractor: Send + Sync {
    async fn validate_swap_parameters(
        &self,
        amount_str: &str,
        source_token: &str,
        target_token: &str,
        slippage_str: Option<&str>,
    ) -> Result<(f64, String, String, f64)>;

    async fn execute_swap(
        &self,
        telegram_id: i64,
        amount: f64,
        source_token: &str,
        target_token: &str,
        slippage: f64,
    ) -> Result<SwapResult>;
}

pub struct SwapInteractorImpl<T, Q>
where
    T: TokenRepository,
    Q: QuoteService,
{
    db_pool: Arc<PgPool>,
    solana_client: Arc<RpcClient>,
    swap_service: Arc<SwapService<T, Q>>,
    token_repository: Arc<dyn TokenRepository + Send + Sync>,
}

impl<T, Q> SwapInteractorImpl<T, Q>
where
    T: TokenRepository + 'static,
    Q: QuoteService + 'static,
{
    pub fn new(
        db_pool: Arc<PgPool>,
        solana_client: Arc<RpcClient>,
        swap_service: Arc<SwapService<T, Q>>,
        token_repository: Arc<dyn TokenRepository + Send + Sync>,
    ) -> Self {
        Self {
            db_pool,
            solana_client,
            swap_service,
            token_repository,
        }
    }
}

#[async_trait]
impl<T, Q> SwapInteractor for SwapInteractorImpl<T, Q>
where
    T: TokenRepository + Send + Sync + 'static,
    Q: QuoteService + Send + Sync + 'static,
{
    async fn validate_swap_parameters(
        &self,
        amount_str: &str,
        source_token: &str,
        target_token: &str,
        slippage_str: Option<&str>,
    ) -> Result<(f64, String, String, f64)> {
        // Parse amount
        let amount = amount_str
            .parse::<f64>()
            .map_err(|_| anyhow!("Invalid amount format"))?;

        if amount <= 0.0 {
            return Err(anyhow!("Amount must be greater than zero"));
        }

        // todo: Validate tokens

        if source_token == target_token {
            return Err(anyhow!("Source and target tokens must be different"));
        }

        // Parse slippage (optional)
        let slippage = if let Some(slippage_text) = slippage_str {
            if slippage_text.ends_with('%') && slippage_text.len() > 1 {
                let slippage_value = slippage_text
                    .trim_end_matches('%')
                    .parse::<f64>()
                    .unwrap_or(0.5);
                slippage_value / 100.0 // Convert percentage to decimal
            } else {
                0.005 // Default 0.5%
            }
        } else {
            0.005 // Default 0.5%
        };

        // Limit slippage range
        let slippage = slippage.max(0.001).min(0.05);

        Ok((
            amount,
            source_token.to_string(),
            target_token.to_string(),
            slippage,
        ))
    }

    async fn execute_swap(
        &self,
        telegram_id: i64,
        amount: f64,
        source_token: &str,
        target_token: &str,
        slippage: f64,
    ) -> Result<SwapResult> {
        // Get user wallet info
        let user = db::get_user_by_telegram_id(&self.db_pool, telegram_id).await?;

        let (address, keypair_base58) = match (user.solana_address, user.encrypted_private_key) {
            (Some(addr), Some(key)) => (addr, key),
            _ => return Err(BotError::WalletNotFound.into()),
        };

        // Get quote
        let quote = match self
            .swap_service
            .get_swap_quote(amount, source_token, target_token, slippage)
            .await
        {
            Ok(q) => q,
            Err(e) => {
                return Ok(SwapResult {
                    source_token: source_token.to_string(),
                    target_token: target_token.to_string(),
                    amount_in: amount,
                    amount_out: 0.0,
                    signature: None,
                    success: false,
                    error_message: Some(format!("Failed to get quote: {}", e)),
                });
            }
        };

        // Get target token info
        let target_token_info = match self.token_repository.get_token_by_id(target_token).await {
            Ok(info) => info,
            Err(e) => {
                return Ok(SwapResult {
                    source_token: source_token.to_string(),
                    target_token: target_token.to_string(),
                    amount_in: amount,
                    amount_out: 0.0,
                    signature: None,
                    success: false,
                    error_message: Some(format!("Failed to get token info: {}", e)),
                });
            }
        };

        let out_amount_raw: f64 = quote.out_amount as f64;

        // Apply correct decimals
        let out_amount = out_amount_raw / 10f64.powi(target_token_info.decimals as i32);

        // Prepare and get swap transaction
        let swap_response = match self
            .swap_service
            .prepare_swap(amount, source_token, target_token, slippage, &address)
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                return Ok(SwapResult {
                    source_token: source_token.to_string(),
                    target_token: target_token.to_string(),
                    amount_in: amount,
                    amount_out: out_amount,
                    signature: None,
                    success: false,
                    error_message: Some(format!("Failed to prepare swap: {}", e)),
                });
            }
        };

        // Get keypair
        let keypair = match solana::keypair_from_base58(&keypair_base58) {
            Ok(kp) => kp,
            Err(e) => {
                return Ok(SwapResult {
                    source_token: source_token.to_string(),
                    target_token: target_token.to_string(),
                    amount_in: amount,
                    amount_out: out_amount,
                    signature: None,
                    success: false,
                    error_message: Some(format!("Failed to parse keypair: {}", e)),
                });
            }
        };

        // Execute swap transaction
        match self
            .swap_service
            .execute_swap_transaction(&self.solana_client, &keypair, &swap_response)
            .await
        {
            Ok(signature) => {
                // Record transaction in database
                let _ = db::record_swap(
                    &self.db_pool,
                    telegram_id,
                    source_token,
                    target_token,
                    amount,
                    out_amount,
                    &Some(signature.clone()),
                    "SUCCESS",
                )
                .await;

                Ok(SwapResult {
                    source_token: source_token.to_string(),
                    target_token: target_token.to_string(),
                    amount_in: amount,
                    amount_out: out_amount,
                    signature: Some(signature),
                    success: true,
                    error_message: None,
                })
            }
            Err(e) => {
                // Record failed transaction
                let _ = db::record_swap(
                    &self.db_pool,
                    telegram_id,
                    source_token,
                    target_token,
                    amount,
                    out_amount,
                    &None::<String>,
                    "FAILED",
                )
                .await;

                Ok(SwapResult {
                    source_token: source_token.to_string(),
                    target_token: target_token.to_string(),
                    amount_in: amount,
                    amount_out: out_amount,
                    signature: None,
                    success: false,
                    error_message: Some(format!("Failed to execute swap: {}", e)),
                })
            }
        }
    }
}
