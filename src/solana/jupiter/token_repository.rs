// src/solana/jupiter/token_repository.rs
use crate::solana::jupiter::models::Token;
use crate::solana::jupiter::{JupiterToken, SOL_MINT, USDC_MINT};
use anyhow::{anyhow, Result};
use log::{error, info, warn};
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex};
use teloxide::payloads::SendVenueSetters;

// Константа для API-эндпоинта
fn token_list_url() -> String {
    env::var("TOKEN_LIST_URL").unwrap_or_else(|_| "https://token.jup.ag/strict".to_string())
}

/// Репозиторий для работы с токенами
pub struct TokenRepository {
    pub http_client: Client,
    token_cache: Arc<Mutex<HashMap<String, Token>>>,
}

impl TokenRepository {
    /// Создает новый экземпляр репозитория
    pub fn new() -> Self {
        Self {
            http_client: Client::new(),
            token_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Получает список всех токенов
    pub async fn get_all_tokens(&self) -> Result<Vec<Token>> {
        let url = token_list_url();

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

        let tokens: Vec<Token> = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse token list response: {}", e))?;

        // Обновляем кеш
        let mut cache = self.token_cache.lock().unwrap();
        for token in &tokens {
            cache.insert(token.id.clone(), token.clone());
        }

        Ok(tokens)
    }

    /// Получает информацию о токене по его ID
    pub async fn get_token_by_id(&mut self, token_id: &str) -> Result<Token> {
        info!("Getting token by ID: {}", token_id);

        // Запрашиваем токен через API
        let url = format!("https://api.jup.ag/tokens/v1/token/{}", token_id);

        let response = self.http_client.get(&url).send().await.map_err(|e| {
            error!("Failed to fetch token from Jupiter API: {}", e);
            anyhow!("Failed to fetch token from API: {}", e)
        })?;

        info!(
            "Jupiter API response: {} for token {}",
            response.status(),
            token_id
        );
        if !response.status().is_success() {
            // Если это SOL или USDC, вернем заглушку
            if token_id == SOL_MINT {
                let sol = Token {
                    id: SOL_MINT.to_string(),
                    symbol: "SOL".to_string(),
                    name: "Solana".to_string(),
                    decimals: 9,
                    logo_uri: "".to_string(),
                };

                return Ok(sol);
            } else if token_id == USDC_MINT {
                let usdc = Token {
                    id: USDC_MINT.to_string(),
                    symbol: "USDC".to_string(),
                    name: "USD Coin".to_string(),
                    decimals: 6,
                    logo_uri: "".to_string(),
                };

                return Ok(usdc);
            }

            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("Jupiter API error [get_token_by_id]: {}", error_text);
            return Err(anyhow!("Jupiter API error: {}", error_text));
        }

        // Парсим ответ
        let jupiter_token: JupiterToken = response.json().await.map_err(|e| {
            error!("Failed to parse token response: {}", e);
            anyhow!("Failed to parse token response: {}", e)
        })?;

        // Преобразуем в наш формат токена
        let token = Token {
            id: jupiter_token.address,
            symbol: jupiter_token.symbol,
            name: jupiter_token.name,
            decimals: jupiter_token.decimals,
            logo_uri: jupiter_token.logo_uri.unwrap_or_default(),
        };

        Ok(token)
    }

    /// Поиск токена по символу
    pub async fn find_token_by_symbol(&self, symbol: &str) -> Result<Token> {
        // Пытаемся найти в кеше
        {
            let cache = self.token_cache.lock().unwrap();
            for token in cache.values() {
                if token.symbol.to_uppercase() == symbol.to_uppercase() {
                    return Ok(token.clone());
                }
            }
        }

        // Если не в кеше, запрашиваем все токены
        let tokens = self.get_all_tokens().await?;

        for token in &tokens {
            if token.symbol.to_uppercase() == symbol.to_uppercase() {
                return Ok(token.clone());
            }
        }

        Err(anyhow!("Token not found: {}", symbol))
    }
}
