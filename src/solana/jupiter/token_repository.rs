use crate::solana::jupiter::models::Token;
use crate::solana::jupiter::{JupiterToken, SOL_MINT, USDC_MINT};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use log::{error, info, warn};
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex};
use teloxide::payloads::SendVenueSetters;

/// Репозиторий для работы с токенами
#[async_trait]
pub trait TokenRepository: Send + Sync {
    async fn get_token_by_id(&self, token_id: &str) -> Result<Token>;
}

/// Реализация репозитория для работы с токенами Jupiter
pub struct JupiterTokenRepository {
    http_client: Client,
    token_cache: Arc<Mutex<HashMap<String, Token>>>,
}

impl JupiterTokenRepository {
    /// Создает новый экземпляр репозитория для Jupiter
    pub fn new() -> Self {
        Self {
            http_client: Client::new(),
            token_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl TokenRepository for JupiterTokenRepository {
    /// Получает информацию о токене по его ID
    async fn get_token_by_id(&self, token_id: &str) -> Result<Token> {
        info!("Getting token by ID: {}", token_id);

        // Запрашиваем токен через API
        let url = format!("https://api.jup.ag/tokens/v1/token/{}", token_id);

        let response = self.http_client.get(&url).send().await.map_err(|e| {
            error!("Failed to fetch token from Jupiter API: {}", e);
            anyhow!("Failed to fetch token from API: {}", e)
        })?;

        info!(
            "Jupiter API response: {} for token {}",
            &response.status(),
            token_id
        );
        if !&response.status().is_success() {
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

            let error_text = response.text().await.unwrap_or("Unknown error".to_string());
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
}
