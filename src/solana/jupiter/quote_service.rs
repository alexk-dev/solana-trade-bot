use anyhow::{anyhow, Result};
use log::{debug, info};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use crate::solana::jupiter::{TokenRepository, TokenService};
use jupiter_swap_api_client::quote::{QuoteRequest, QuoteResponse};
use jupiter_swap_api_client::JupiterSwapApiClient;
use reqwest::Client;

/// Сервис для получения котировок обмена токенов
pub struct QuoteService {
    pub token_repository: TokenRepository,
    jupiter_client: JupiterSwapApiClient,
}

impl QuoteService {
    /// Создает новый экземпляр сервиса котировок
    pub fn new() -> Self {
        Self {
            token_repository: TokenRepository::new(),
            jupiter_client: JupiterSwapApiClient::new("https://quote-api.jup.ag/v6".to_string()),
        }
    }

    /// Получает котировку для обмена токенов
    pub async fn get_swap_quote(
        &mut self,
        amount: f64,
        source_token: &str,
        target_token: &str,
        slippage: f64,
    ) -> Result<QuoteResponse> {
        // Получаем информацию о токенах для определения decimals
        let source_token_info = self.token_repository.get_token_by_id(source_token).await?;

        // Конвертируем amount с учетом decimals
        let decimals = source_token_info.decimals as u32;
        let amount_in = (amount * 10f64.powi(decimals as i32)) as u64;

        // Конвертируем slippage в базисные пункты
        let slippage_bps = (slippage * 10000.0) as u16;

        // Парсим строковые адреса токенов в Pubkey
        let input_mint = Pubkey::from_str(source_token)
            .map_err(|e| anyhow!("Invalid source token address: {}", e))?;

        let output_mint = Pubkey::from_str(target_token)
            .map_err(|e| anyhow!("Invalid target token address: {}", e))?;

        // Создаем запрос котировки через SDK
        let quote_request = QuoteRequest {
            amount: amount_in,
            input_mint,
            output_mint,
            slippage_bps,
            ..QuoteRequest::default()
        };

        debug!("Requesting quote with parameters: {:?}", quote_request);

        // Отправляем запрос через SDK
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
