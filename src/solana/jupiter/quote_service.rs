use crate::solana::jupiter::token_repository::TokenRepository;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use jupiter_swap_api_client::quote::{QuoteRequest, QuoteResponse};
use jupiter_swap_api_client::JupiterSwapApiClient;
use log::{debug, info};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Service for getting token exchange quotes
#[async_trait]
pub trait QuoteService: Send + Sync {
    async fn get_swap_quote(
        &self,
        amount: f64,
        source_token: &str,
        target_token: &str,
        slippage: f64,
    ) -> Result<QuoteResponse>;
}

pub struct JupiterQuoteService<T: TokenRepository> {
    pub token_repository: T,
    pub jupiter_client: JupiterSwapApiClient,
}

impl<T: TokenRepository> JupiterQuoteService<T> {
    /// Creates a new quote service instance
    pub fn new(token_repository: T) -> Self {
        Self {
            token_repository,
            jupiter_client: JupiterSwapApiClient::new("https://quote-api.jup.ag/v6".to_string()),
        }
    }
}

#[async_trait]
impl<T: TokenRepository + Send + Sync> QuoteService for JupiterQuoteService<T> {
    /// Gets a quote for token exchange
    async fn get_swap_quote(
        &self,
        amount: f64,
        source_token: &str,
        target_token: &str,
        slippage: f64,
    ) -> Result<QuoteResponse> {
        // Get token information to determine decimals
        let source_token_info = &self
            .token_repository
            .get_token_by_id(&source_token.to_string())
            .await?;

        // Convert amount considering decimals
        let decimals = source_token_info.decimals as u32;
        let amount_in = (amount * 10f64.powi(decimals as i32)) as u64;

        // Convert slippage to basis points
        let slippage_bps = (slippage * 10000.0) as u16;

        // Parse token addresses to Pubkey
        let input_mint = Pubkey::from_str(source_token)
            .map_err(|e| anyhow!("Invalid source token address: {}", e))?;

        let output_mint = Pubkey::from_str(target_token)
            .map_err(|e| anyhow!("Invalid target token address: {}", e))?;

        // Create quote request via SDK
        let quote_request = QuoteRequest {
            amount: amount_in,
            input_mint,
            output_mint,
            slippage_bps,
            ..QuoteRequest::default()
        };

        debug!("Requesting quote with parameters: {:?}", quote_request);

        // Send request via SDK
        let quote_response = self
            .jupiter_client
            .quote(&quote_request)
            .await
            .map_err(|e| anyhow!("Failed to get quote from Jupiter API: {}", e))?;

        info!(
            "Quote received successfully: input_amount={}, output_amount={}",
            quote_response.in_amount, quote_response.out_amount
        );

        Ok(quote_response)
    }
}
