use anyhow::{anyhow, Result};
use async_trait::async_trait;
use log::info;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::solana::jupiter::models::{Token, TokenPrice};
use crate::solana::jupiter::quote_service::QuoteService;
use crate::solana::jupiter::token_repository::TokenRepository;
use crate::solana::jupiter::Config;

// Структура для обработки ошибок из Jupiter API
#[derive(Deserialize)]
struct ErrorResponse {
    error: String,
}

/// Интерфейс для сервиса информации о ценах токенов
#[async_trait]
pub trait PriceService: Send + Sync {
    /// Получить текущую цену SOL в USDC
    async fn get_sol_price(&self) -> Result<f64>;

    /// Получить цену токена в SOL и USDC
    async fn get_token_price(&self, token_id: &str) -> Result<TokenPrice>;

    /// Получить цены для множества токенов
    async fn get_prices(&self, vs_token: Option<&str>) -> Result<HashMap<String, f64>>;
}

/// Реализация сервиса цен с использованием Jupiter API
pub struct JupiterPriceService<T: TokenRepository, Q: QuoteService> {
    token_repository: T,
    quote_service: Q,
    http_client: Client,
    config: Config,
    sol_usdc_price: f64,
}

impl<T: TokenRepository, Q: QuoteService> JupiterPriceService<T, Q> {
    /// Создает новый экземпляр сервиса цен с внедрением зависимостей
    pub fn new(token_repository: T, quote_service: Q, config: Config) -> Self {
        Self {
            token_repository,
            quote_service,
            http_client: Client::new(),
            config,
            sol_usdc_price: 0.0, // Будет обновлено при первом вызове
        }
    }

    /// Обновляет кэшированное значение цены SOL в USDC
    async fn refresh_sol_price(&self) -> Result<f64> {
        // Получаем котировку с использованием QuoteService
        let quote = self
            .quote_service
            .get_swap_quote(
                1.0,
                &self.config.sol_token_address,
                &self.config.usdc_token_address,
                0.5,
            )
            .await?;

        // Конвертируем в USDC с учетом decimals (6)
        let sol_price_in_usdc = quote.out_amount as f64 / 1_000_000.0;

        Ok(sol_price_in_usdc)
    }

    /// Проверяет ответ API на наличие ошибки
    fn check_for_api_error<D>(&self, value: serde_json::Value) -> Result<D>
    where
        D: serde::de::DeserializeOwned,
    {
        if let Ok(ErrorResponse { error }) = serde_json::from_value::<ErrorResponse>(value.clone())
        {
            Err(anyhow!("API error: {}", error))
        } else {
            serde_json::from_value(value)
                .map_err(|err| anyhow!("JSON deserialization error: {}", err))
        }
    }
}

#[async_trait]
impl<T: TokenRepository + Send + Sync, Q: QuoteService + Send + Sync> PriceService
    for JupiterPriceService<T, Q>
{
    /// Получить текущую цену SOL в USDC
    async fn get_sol_price(&self) -> Result<f64> {
        let sol_price = self.refresh_sol_price().await?;

        Ok(sol_price)
    }

    /// Получить цену токена в SOL и USDC
    async fn get_token_price(&self, token_id: &str) -> Result<TokenPrice> {
        // Если запрашиваем цену SOL, возвращаем известные значения
        if token_id == self.config.sol_token_address {
            let sol_price = self.get_sol_price().await?;

            return Ok(TokenPrice {
                token_id: self.config.sol_token_address.clone(),
                symbol: "SOL".to_string(),
                price_in_sol: 1.0,
                price_in_usdc: sol_price,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            });
        }

        // Получаем информацию о токене
        let token = self.token_repository.get_token_by_id(token_id).await?;

        // Получаем котировку для обмена 1 единицы токена на SOL
        let quote = self
            .quote_service
            .get_swap_quote(
                1.0,
                token_id,
                &self.config.sol_token_address,
                0.5, // 0.5% slippage
            )
            .await?;

        // Конвертируем в SOL с учетом decimals (9)
        let price_in_sol = quote.out_amount as f64 / 1_000_000_000.0;

        // Получаем текущую цену SOL/USDC если нужно
        let sol_usdc_price = self.get_sol_price().await?;

        // Расчитываем цену в USDC
        let price_in_usdc = price_in_sol * sol_usdc_price;

        Ok(TokenPrice {
            token_id: token_id.to_string(),
            symbol: token.symbol,
            price_in_sol,
            price_in_usdc,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }

    /// Получить цены для множества токенов
    async fn get_prices(&self, vs_token: Option<&str>) -> Result<HashMap<String, f64>> {
        let url = match vs_token {
            Some(token) => format!("{}/price?vsToken={}", self.config.price_api_url, token),
            None => format!("{}/price", self.config.price_api_url),
        };

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Jupiter API error: {}", error_text));
        }

        // Парсим JSON ответ
        let price_data: HashMap<String, f64> = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse prices response: {}", e))?;

        Ok(price_data)
    }
}
