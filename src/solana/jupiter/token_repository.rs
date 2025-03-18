use std::collections::HashMap;
use log::{info, warn, error};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use crate::solana::jupiter::token::{Token, SOL_MINT, USDC_MINT};

// Хранилище токенов, которое работает с API
pub struct TokenRepository {
    http_client: reqwest::Client,
    // Кэш для часто используемых токенов
    cache: HashMap<String, Token>,
    // Кэш для цен токенов
    price_cache: HashMap<String, TokenPrice>,
}

#[derive(Debug, Deserialize)]
struct JupiterToken {
    address: String,
    symbol: String,
    name: String,
    decimals: u8,
    #[serde(rename = "logoURI")]
    logo_uri: Option<String>,
}

// Структура для ответа API цен Jupiter
#[derive(Debug, Deserialize)]
struct JupiterPriceResponse {
    data: HashMap<String, TokenData>,
    #[serde(rename = "timeTaken")]
    time_taken: f64,
    #[serde(rename = "responseCode")]
    response_code: i32,
}

#[derive(Debug, Clone, Deserialize)]
struct TokenData {
    id: String,
    #[serde(rename = "type")]
    token_type: String,
    price: f64,
}

// Структура для хранения цены токена
#[derive(Debug, Clone)]
pub struct TokenPrice {
    pub id: String,
    pub price: f64,
    pub timestamp: u64,
}

impl TokenRepository {
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
            cache: HashMap::new(),
            price_cache: HashMap::new(),
        }
    }

    // Получить токен по ID (mint адресу)
    pub async fn get_token_by_id(&mut self, id: &str) -> Result<Token> {
        info!("Getting token by ID: {}", id);

        // Проверяем кэш
        if let Some(token) = self.cache.get(id) {
            return Ok(token.clone());
        }

        // Запрашиваем токен через API
        let url = format!("https://api.jup.ag/tokens/v1/token/{}", id);

        let response = self.http_client.get(&url)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to fetch token from Jupiter API: {}", e);
                anyhow!("Failed to fetch token from API: {}", e)
            })?;

        info!("Jupiter API response: {} for token {}", response.status(), id);
        if !response.status().is_success() {
            // Если это SOL или USDC, вернем заглушку
            if id == SOL_MINT {
                let sol = Token {
                    id: SOL_MINT.to_string(),
                    symbol: "SOL".to_string(),
                    name: "Solana".to_string(),
                    decimals: 9,
                    logo_uri: "".to_string(),
                };
                self.cache.insert(id.to_string(), sol.clone());
                return Ok(sol);
            } else if id == USDC_MINT {
                let usdc = Token {
                    id: USDC_MINT.to_string(),
                    symbol: "USDC".to_string(),
                    name: "USD Coin".to_string(),
                    decimals: 6,
                    logo_uri: "".to_string(),
                };
                self.cache.insert(id.to_string(), usdc.clone());
                return Ok(usdc);
            }

            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("Jupiter API error [get_token_by_id]: {}", error_text);
            return Err(anyhow!("Jupiter API error: {}", error_text));
        }

        // Парсим ответ
        let jupiter_token: JupiterToken = response.json().await
            .map_err(|e| {
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

        // Кэшируем результат
        self.cache.insert(id.to_string(), token.clone());

        Ok(token)
    }

    // Получить символ токена по ID (mint)
    pub async fn get_symbol_from_id(&mut self, id: &str) -> Result<String> {
        let token = self.get_token_by_id(id).await?;
        Ok(token.symbol)
    }

    // Получить цены для списка токенов
    pub async fn get_prices(&mut self, token_ids: &[String]) -> Result<HashMap<String, TokenPrice>> {
        if token_ids.is_empty() {
            return Ok(HashMap::new());
        }

        // Составляем URL запроса с разделенными запятыми ID токенов
        let ids_param = token_ids.join(",");
        let url = format!("https://api.jup.ag/price/v2?ids={}", ids_param);

        info!("Fetching prices for {} tokens: {}", token_ids.len(), url);

        // Выполняем запрос
        let response = self.http_client.get(&url)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to fetch prices from Jupiter API: {}", e);
                anyhow!("Failed to fetch prices from API: {}", e)
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("Jupiter API error [get_prices]: {}", error_text);
            return Err(anyhow!("Jupiter API error: {}", error_text));
        }

        // Парсим ответ
        let price_response: JupiterPriceResponse = response.json().await
            .map_err(|e| {
                error!("Failed to parse price response: {}", e);
                anyhow!("Failed to parse price response: {}", e)
            })?;

        // Проверяем код ответа
        if price_response.response_code != 200 {
            return Err(anyhow!(
                "Jupiter API returned non-OK response code: {}",
                price_response.response_code
            ));
        }

        // Текущее время для метки времени кэширования
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Преобразуем ответ в наш формат и обновляем кэш
        let mut result = HashMap::new();
        for (id, token_data) in price_response.data {
            let price = TokenPrice {
                id: token_data.id.clone(),
                price: token_data.price,
                timestamp: current_time,
            };

            // Обновляем кэш цен
            self.price_cache.insert(token_data.id.clone(), price.clone());

            // Добавляем в результат
            result.insert(id, price);
        }

        Ok(result)
    }

    // Получить цену для одного токена
    pub async fn get_price(&mut self, token_id: &str) -> Result<TokenPrice> {
        // Проверяем кэш
        if let Some(price) = self.price_cache.get(token_id) {
            // Проверяем, не устарела ли цена (старше 5 минут)
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            if current_time - price.timestamp < 300 { // 5 минут = 300 секунд
                return Ok(price.clone());
            }
        }

        // Если кэша нет или он устарел, запрашиваем новую цену
        let prices = self.get_prices(&[token_id.to_string()]).await?;

        if let Some(price) = prices.get(token_id) {
            Ok(price.clone())
        } else {
            Err(anyhow!("Price for token {} not found", token_id))
        }
    }

    // Очистить кэш цен
    pub fn clear_price_cache(&mut self) {
        self.price_cache.clear();
    }

    // Очистить кэш токенов
    pub fn clear_token_cache(&mut self) {
        self.cache.clear();
    }
}