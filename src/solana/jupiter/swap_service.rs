use std::collections::HashMap;
// src/solana/jupiter/swap_service.rs
use anyhow::{anyhow, Result};
use log::{debug, error, info};
use reqwest::Client;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use std::env;
use std::str::FromStr;
use std::sync::Arc;

use crate::solana::jupiter::{models::TokenPrice, SwapRequest, TokenService};
use jupiter_swap_api_client::{
    quote::QuoteRequest, swap::SwapRequest as JupiterSwapRequest,
    transaction_config::TransactionConfig, JupiterSwapApiClient,
};
use solana_sdk::signature::NullSigner;
use solana_sdk::transaction::VersionedTransaction;

/// Сервис для выполнения операций свопа с использованием Jupiter
pub struct SwapService {
    pub token_service: TokenService,
    http_client: Client,
    jupiter_client: JupiterSwapApiClient,
}

impl SwapService {
    /// Создает новый экземпляр сервиса свопа с использованием официального SDK
    pub fn new() -> Self {
        Self {
            token_service: TokenService::new(),
            http_client: Client::new(),
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
    ) -> Result<jupiter_swap_api_client::quote::QuoteResponse> {
        // Получаем информацию о токенах для определения decimals
        let source_token_info = self
            .token_service
            .token_repository
            .get_token_by_id(source_token)
            .await?;

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

    /// Подготавливает и получает транзакцию свопа
    pub async fn prepare_swap(
        &mut self,
        amount: f64,
        source_token: &str,
        target_token: &str,
        slippage: f64,
        user_public_key: &str,
    ) -> Result<jupiter_swap_api_client::swap::SwapResponse> {
        // Получаем котировку
        debug!(
            "Getting swap quote for {} {} to {}",
            amount, source_token, target_token
        );
        let quote_response = self
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
        swap_response: &jupiter_swap_api_client::swap::SwapResponse,
    ) -> Result<String> {
        info!("Executing swap transaction");

        // Теперь мы используем транзакцию напрямую из SDK
        // let raw_transaction = swap_response.swap_transaction.clone();

        // Отправляем транзакцию в сеть
        // let signature = match solana_client.send_transaction_with_config(
        //     &raw_transaction,
        //     solana_client::rpc_config::RpcSendTransactionConfig {
        //         skip_preflight: true,
        //         preflight_commitment: None,
        //         encoding: None,
        //         max_retries: Some(5),
        //         min_context_slot: None,
        //     }
        // ).await {
        //     Ok(sig) => sig,
        //     Err(e) => {
        //         error!("Failed to send transaction: {}", e);
        //         return Err(anyhow!("Failed to send transaction: {}", e));
        //     }
        // };
        //
        // info!("Transaction sent successfully: {}", signature);
        //
        // // Возвращаем подпись транзакции
        // Ok(signature.to_string())
        println!("Raw tx len: {}", swap_response.swap_transaction.len());

        let versioned_transaction: VersionedTransaction =
            bincode::deserialize(&swap_response.swap_transaction).unwrap();

        // Replace with a keypair or other struct implementing signer
        let signed_versioned_transaction =
            VersionedTransaction::try_new(versioned_transaction.message, &[&keypair]).unwrap();

        // send with rpc client...
        let rpc_client = RpcClient::new("https://api.devnet.solana.com".into());

        info!("Calling network");

        let signature = rpc_client
            .send_and_confirm_transaction(&signed_versioned_transaction)
            .await?;

        println!("{signature}");

        // // POST /swap-instructions
        // let swap_instructions = self.jupiter_client
        //     .swap_instructions(&SwapRequest {
        //         user_public_key: keypair,
        //         quote_response,
        //         config: TransactionConfig::default(),
        //     })
        //     .await
        //     .unwrap();
        // println!("swap_instructions: {swap_instructions:?}");

        Ok(signature.to_string())
    }

    /// Получает аудит транзакции свопа
    pub async fn get_swap_instructions(
        &mut self,
        amount: f64,
        source_token: &str,
        target_token: &str,
        slippage: f64,
        user_public_key: &str,
    ) -> Result<jupiter_swap_api_client::swap::SwapInstructionsResponse> {
        // Получаем котировку
        let quote_response = self
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
