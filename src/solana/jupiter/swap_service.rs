use crate::solana::jupiter::quote_service::QuoteService;
use crate::solana::jupiter::token_repository::TokenRepository;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use jupiter_swap_api_client::{
    quote::QuoteResponse,
    swap::{SwapInstructionsResponse, SwapRequest as JupiterSwapRequest, SwapResponse},
    transaction_config::TransactionConfig,
    JupiterSwapApiClient,
};
use log::{debug, error, info};
use reqwest::Client as HttpClient;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::transaction::VersionedTransaction;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

/// Сервис для выполнения операций свопа с использованием Jupiter
pub struct SwapService<T: TokenRepository, Q: QuoteService> {
    token_repository: T,
    quote_service: Q,
    jupiter_client: JupiterSwapApiClient,
}

impl<T: TokenRepository, Q: QuoteService> SwapService<T, Q> {
    /// Создает новый экземпляр сервиса свопа с использованием официального SDK
    pub fn new(token_repository: T, quote_service: Q) -> Self {
        Self {
            token_repository,
            quote_service,
            jupiter_client: JupiterSwapApiClient::new("https://quote-api.jup.ag/v6".to_string()),
        }
    }

    /// Подготавливает и получает транзакцию свопа
    pub async fn prepare_swap(
        &self,
        amount: f64,
        source_token: &str,
        target_token: &str,
        slippage: f64,
        user_public_key: &str,
    ) -> Result<SwapResponse> {
        // Получаем котировку
        debug!(
            "Getting swap quote for {} {} to {}",
            amount, source_token, target_token
        );
        let quote_response = &self
            .quote_service
            .get_swap_quote(amount, source_token, target_token, slippage)
            .await?;

        // Парсим pubkey пользователя
        let user_pubkey = Pubkey::from_str(user_public_key)
            .map_err(|e| anyhow!("Invalid user public key: {}", e))?;

        // Создаем запрос свопа
        let swap_request = JupiterSwapRequest {
            user_public_key: user_pubkey,
            quote_response: quote_response.clone(),
            config: TransactionConfig::default(),
        };

        debug!(
            "Requesting swap transaction with user_public_key: {}",
            user_public_key
        );

        // Получаем транзакцию свопа через SDK
        let swap_response = self
            .jupiter_client
            .swap(&swap_request, Some(HashMap::new()))
            .await
            .map_err(|e| anyhow!("Failed to get swap transaction: {}", e))?;

        info!(
            "Swap transaction received: tx_length={}",
            swap_response.swap_transaction.len()
        );

        Ok(swap_response)
    }

    /// Выполняет (подписывает и отправляет) транзакцию свопа в сеть
    pub async fn execute_swap_transaction(
        &self,
        solana_client: &Arc<RpcClient>,
        keypair: &Keypair,
        swap_response: &SwapResponse,
    ) -> Result<String> {
        info!("Executing swap transaction");
        println!("Raw tx len: {}", swap_response.swap_transaction.len());

        let versioned_transaction: VersionedTransaction =
            bincode::deserialize(&swap_response.swap_transaction)
                .map_err(|e| anyhow!("Failed to deserialize transaction: {}", e))?;

        // Подписываем транзакцию
        let signed_versioned_transaction =
            VersionedTransaction::try_new(versioned_transaction.message, &[keypair])
                .map_err(|e| anyhow!("Failed to sign transaction: {}", e))?;

        info!("Calling network");

        let signature = solana_client
            .send_and_confirm_transaction(&signed_versioned_transaction)
            .await
            .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;

        println!("Transaction signature: {}", signature);

        Ok(signature.to_string())
    }

    /// Получает аудит транзакции свопа
    pub async fn get_swap_instructions(
        &self,
        amount: f64,
        source_token: &str,
        target_token: &str,
        slippage: f64,
        user_public_key: &str,
    ) -> Result<SwapInstructionsResponse> {
        // Получаем котировку
        let quote_response = self
            .quote_service
            .get_swap_quote(amount, source_token, target_token, slippage)
            .await?;

        // Парсим pubkey пользователя
        let user_pubkey = Pubkey::from_str(user_public_key)
            .map_err(|e| anyhow!("Invalid user public key: {}", e))?;

        // Создаем запрос на инструкции свопа
        let swap_request = JupiterSwapRequest {
            user_public_key: user_pubkey,
            quote_response,
            config: TransactionConfig::default(),
        };

        // Получаем инструкции свопа через SDK
        let swap_instructions = self
            .jupiter_client
            .swap_instructions(&swap_request)
            .await
            .map_err(|e| anyhow!("Failed to get swap instructions: {}", e))?;

        Ok(swap_instructions)
    }
}
