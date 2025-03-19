// src/solana/jupiter/swap_service.rs
use std::env;
use anyhow::{anyhow, Result};
use log::{info, debug, error};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    signature::Keypair,
    transaction::VersionedTransaction,
    signature::Signer,
};
use std::sync::Arc;
use base64::engine::{Engine as _, general_purpose::STANDARD as BASE64};
use solana_transaction_status::UiTransactionEncoding;
use reqwest::Client;
use serde_json::json;

use crate::solana::jupiter::{
    models::*,
    TokenService,
};

// Константы для API-эндпоинтов
fn quote_api_url() -> String {
    env::var("QUOTE_API_URL").unwrap_or_else(|_| "https://quote-api.jup.ag/v6".to_string())
}

/// Сервис для выполнения операций свопа с использованием Jupiter
pub struct SwapService {
    pub token_service: TokenService,
    http_client: Client,
}

impl SwapService {
    /// Создает новый экземпляр сервиса свопа
    pub fn new() -> Self {
        Self {
            token_service: TokenService::new(),
            http_client: Client::new(),
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
        self.token_service.get_swap_quote(amount, source_token, target_token, slippage).await
    }

    /// Создает запрос для свопа с указанными параметрами
    pub fn create_swap_request(
        &self,
        quote: QuoteResponse,
        user_public_key: &str,
        destination_token_account: Option<&str>,
    ) -> SwapRequest {
        SwapRequest {
            user_public_key: user_public_key.to_string(),
            wrap_and_unwrap_sol: Some(true),
            use_shared_accounts: Some(true),
            fee_account: None,
            prioritization_fee_lamports: PrioritizationFeeLamportsWrapper::Auto { auto: true },
            as_legacy_transaction: Some(false),
            use_token_ledger: Some(false),
            destination_token_account: destination_token_account.map(|s| s.to_string()),
            quote_response: quote,
        }
    }

    /// Получает транзакцию для свопа от Jupiter API
    pub async fn get_swap_transaction(&self, swap_request: SwapRequest) -> Result<SwapResponse> {
        let url = format!("{}/swap", quote_api_url());

        // Логируем запрос с уровнем INFO
        let request_json = serde_json::to_string_pretty(&swap_request)
            .map_err(|e| anyhow!("Failed to serialize swap request: {}", e))?;

        info!("Jupiter Swap API Request: {}", request_json);

        // Отправляем запрос
        let response = self.http_client.post(&url)
            .header("Accept", "application/json")
            .json(&swap_request)
            .send()
            .await
            .map_err(|e| anyhow!("HTTP request failed: {}", e))?;

        // Проверяем статус ответа
        let status = response.status();
        info!("Jupiter API swap response status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("Jupiter API error response: {}", error_text);
            return Err(anyhow!("Jupiter API error: {}", error_text));
        }

        // Получаем тело ответа
        let body_text = response.text().await
            .map_err(|e| anyhow!("Failed to get response text: {}", e))?;

        info!("Jupiter API swap response body: {}", body_text);

        // Пытаемся разобрать JSON
        let json_result = serde_json::from_str::<SwapResponse>(&body_text);

        match json_result {
            Ok(swap_response) => Ok(swap_response),
            Err(e) => {
                error!("Failed to parse swap response: {}", e);

                // Пытаемся проверить, содержит ли ответ поле error
                if let Ok(error_resp) = serde_json::from_str::<serde_json::Value>(&body_text) {
                    if let Some(error) = error_resp.get("error") {
                        return Err(anyhow!("Jupiter API error: {}", error));
                    }
                }

                Err(anyhow!("Failed to deserialize swap response: {}. Response body: {}", e, body_text))
            }
        }
    }

    /// Выполняет весь процесс свопа: получение котировки, создание запроса, получение транзакции
    pub async fn prepare_swap(
        &mut self,
        amount: f64,
        source_token: &str,
        target_token: &str,
        slippage: f64,
        user_public_key: &str,
    ) -> Result<SwapResponse> {
        // Получаем котировку
        debug!("Getting swap quote for {} {} to {}", amount, source_token, target_token);
        let quote = self.get_swap_quote(amount, source_token, target_token, slippage).await?;

        debug!("Received quote: in_amount={}, out_amount={}, price_impact={}%",
        quote.in_amount, quote.out_amount, quote.price_impact_pct);

        // Проверка полученных данных
        if quote.in_amount.is_empty() || quote.out_amount.is_empty() {
            return Err(anyhow!("Invalid quote: empty in_amount or out_amount"));
        }

        // Создаем запрос на свап
        debug!("Creating swap request for user {}", user_public_key);

        // Создаем запрос со стандартными параметрами
        let swap_request = SwapRequest {
            user_public_key: user_public_key.to_string(),
            wrap_and_unwrap_sol: Some(true),
            use_shared_accounts: Some(true),
            fee_account: None,
            prioritization_fee_lamports: PrioritizationFeeLamportsWrapper::Auto { auto: true },
            as_legacy_transaction: Some(false),
            use_token_ledger: Some(false),
            destination_token_account: None,
            quote_response: quote,
        };

        // Проверка на использование SOL и обработка его как особого случая
        let is_sol_involved = source_token.contains("So11111111111111111111111111111111111111112") ||
            target_token.contains("So11111111111111111111111111111111111111112");

        debug!("Is SOL involved in the swap: {}", is_sol_involved);

        // Получаем транзакцию
        debug!("Requesting swap transaction from Jupiter API");

        let mut max_retries = 3;
        let mut last_error = None;

        // Попытки получить транзакцию с повторами при ошибке
        while max_retries > 0 {
            match self.get_swap_transaction(swap_request.clone()).await {
                Ok(swap_response) => {
                    debug!("Swap transaction received successfully");
                    return Ok(swap_response);
                },
                Err(e) => {
                    info!("Failed to get swap transaction (retries left: {}): {}", max_retries - 1, e);
                    last_error = Some(e);
                    max_retries -= 1;
                }
            }
        }

        // Если все попытки завершились неудачей
        Err(last_error.unwrap_or_else(|| anyhow!("Failed to get swap transaction after multiple retries")))
    }

    /// Выполняет (подписывает и отправляет) транзакцию свопа в сеть
    pub async fn execute_swap_transaction(
        &self,
        solana_client: &Arc<RpcClient>,
        keypair: &Keypair,
        swap_response: &SwapResponse
    ) -> Result<String> {
        // Используем метод с дополнительными опциями, установив значения по умолчанию
        self.execute_swap_transaction_with_options(
            solana_client,
            keypair,
            swap_response,
            true, // skip_preflight = true
            Some(5) // max_retries = 5
        ).await
    }

    /// Выполняет (подписывает и отправляет) транзакцию свопа в сеть с дополнительными опциями
    pub async fn execute_swap_transaction_with_options(
        &self,
        solana_client: &Arc<RpcClient>,
        keypair: &Keypair,
        swap_response: &SwapResponse,
        skip_preflight: bool,
        max_retries: Option<usize>
    ) -> Result<String> {
        info!("Executing swap transaction with options");

        // Декодируем base64 строку в байты
        let transaction_data = BASE64.decode(&swap_response.swap_transaction)
            .map_err(|e| anyhow!("Failed to decode transaction data: {}", e))?;

        // Десериализуем в VersionedTransaction
        let mut transaction: VersionedTransaction = bincode::deserialize(&transaction_data)
            .map_err(|e| anyhow!("Failed to deserialize transaction: {}", e))?;

        // Подписываем транзакцию
        transaction.signatures[0] = keypair.sign_message(&transaction.message.serialize());

        // Отправляем транзакцию с конфигурацией
        let signature = solana_client.send_transaction_with_config(
            &transaction,
            solana_client::rpc_config::RpcSendTransactionConfig {
                skip_preflight,
                preflight_commitment: None,
                encoding: Some(UiTransactionEncoding::Base64),
                max_retries,
                min_context_slot: None,
            }
        ).await
            .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;

        // Возвращаем подпись транзакции
        Ok(signature.to_string())
    }
}