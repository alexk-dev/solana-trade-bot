// src/solana/jupiter/token_service.rs
use anyhow::{anyhow, Result};
use log::{info, debug};
use std::collections::HashMap;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use reqwest::Client;
use serde::Deserialize;

use crate::solana::jupiter::models::*;
use crate::solana::jupiter::token_repository::TokenRepository;

// Константы для API-эндпоинтов
fn quote_api_url() -> String {
    env::var("QUOTE_API_URL").unwrap_or_else(|_| "https://quote-api.jup.ag/v6".to_string())
}

fn price_api_url() -> String {
    env::var("PRICE_API_URL").unwrap_or_else(|_| "https://price.jup.ag/v1".to_string())
}

// Структура для обработки ошибок из Jupiter API
#[derive(Deserialize)]
struct ErrorResponse {
    error: String,
}

// Структура для работы с Jupiter и ценами токенов
pub struct TokenService {
    pub token_repository: TokenRepository,
    pub http_client: Client,
    pub sol_usdc_price: f64, // Текущая цена SOL в USDC
}

impl TokenService {
    pub fn new() -> Self {
        Self {
            token_repository: TokenRepository::new(),
            http_client: Client::new(),
            sol_usdc_price: 0.0, // Будет обновлено при первом вызове refresh_sol_price
        }
    }

    // Обновить цену SOL в USDC
    pub async fn refresh_sol_price(&mut self) -> Result<f64> {
        let quote = self.get_swap_quote(1.0, SOL_MINT, USDC_MINT, 0.5).await?;

        // Конвертируем строку outAmount в f64
        let out_amount = quote.out_amount
            .parse::<f64>()
            .map_err(|e| anyhow!("Failed to parse out amount: {}", e))?;

        // Учитываем decimals для USDC (6)
        let sol_price_in_usdc = out_amount / 1_000_000.0;
        self.sol_usdc_price = sol_price_in_usdc;

        Ok(sol_price_in_usdc)
    }

    // Проверить ответ API на наличие ошибки
    fn check_for_api_error<T>(&self, value: serde_json::Value) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        if let Ok(ErrorResponse { error }) = serde_json::from_value::<ErrorResponse>(value.clone()) {
            Err(anyhow!("Jupiter API error: {}", error))
        } else {
            serde_json::from_value(value).map_err(|err| anyhow!("JSON deserialization error: {}", err))
        }
    }

    // Получить котировку свопа по ID токенов (оставляем, так как используется для обновления цены SOL)
    pub async fn get_swap_quote(
        &mut self,
        amount: f64,
        source_token: &str,
        target_token: &str,
        slippage: f64,
    ) -> Result<QuoteResponse> {
        // Получаем информацию о токенах
        let source_token = self.token_repository.get_token_by_id(source_token).await?;
        let target_token = self.token_repository.get_token_by_id(target_token).await?;

        // Конвертируем amount с учетом decimals
        let decimals = source_token.decimals as u32;
        let amount_in = (amount * 10f64.powi(decimals as i32)) as u64;

        // Конвертируем slippage в базисные пункты
        let slippage_bps = (slippage * 10000.0) as u64;

        // Формируем URL запроса
        let url = format!(
            "{base_url}/quote?inputMint={input_mint}&outputMint={output_mint}&amount={amount}&onlyDirectRoutes=false&slippageBps={slippage_bps}",
            base_url = quote_api_url(),
            input_mint = source_token.id,
            output_mint = target_token.id,
            amount = amount_in,
            slippage_bps = slippage_bps,
        );

        // Отправляем запрос
        let response = self.http_client.get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("HTTP request failed: {}", e))?;

        // Проверяем статус
        if !response.status().is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Jupiter API error: {}", error_text));
        }

        // Парсим ответ
        let json_value = response.json::<serde_json::Value>().await
            .map_err(|e| anyhow!("Failed to parse response as JSON: {}", e))?;

        // Проверяем на наличие ошибок в ответе API
        let quote = self.check_for_api_error::<QuoteResponse>(json_value)?;

        Ok(quote)
    }

    // Получить цену токена в SOL и USDC
    pub async fn get_token_price(&mut self, token_id: &str) -> Result<TokenPrice> {
        // Если запрашиваем цену SOL, возвращаем известные значения
        if token_id == SOL_MINT {
            return Ok(TokenPrice {
                token_id: SOL_MINT.to_string(),
                symbol: "SOL".to_string(),
                price_in_sol: 1.0,
                price_in_usdc: self.sol_usdc_price,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            });
        }

        // Проверяем, обновлена ли цена SOL
        if self.sol_usdc_price == 0.0 {
            self.refresh_sol_price().await?;
        }

        // Получаем информацию о токене
        let token = self.token_repository.get_token_by_id(token_id).await?;

        // Получаем котировку для обмена 1 единицы токена на SOL
        let quote = self.get_swap_quote(
            1.0,
            token_id,
            SOL_MINT,
            0.5 // 0.5% slippage
        ).await?;

        // Конвертируем строку outAmount в f64 и учитываем decimals для SOL (9)
        let out_amount = quote.out_amount
            .parse::<f64>()
            .map_err(|e| anyhow!("Failed to parse out amount: {}", e))?;

        let price_in_sol = out_amount / 1_000_000_000.0;

        // Расчитываем цену в USDC, используя известную цену SOL/USDC
        let price_in_usdc = price_in_sol * self.sol_usdc_price;

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

    // Получить доступные маршруты обмена
    pub async fn get_route_map(&self) -> Result<HashMap<String, Vec<String>>> {
        let url = format!(
            "{}/indexed-route-map?onlyDirectRoutes=false",
            quote_api_url()
        );

        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct IndexedRouteMap {
            mint_keys: Vec<String>,
            indexed_route_map: HashMap<usize, Vec<usize>>,
        }

        let response = self.http_client.get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Jupiter API error: {}", error_text));
        }

        let route_map_response = response.json::<IndexedRouteMap>().await
            .map_err(|e| anyhow!("Failed to parse route map response: {}", e))?;

        let mint_keys = route_map_response.mint_keys;
        let mut route_map = HashMap::new();

        for (from_index, to_indices) in route_map_response.indexed_route_map {
            if from_index < mint_keys.len() {
                let from_mint = mint_keys[from_index].clone();
                let to_mints: Vec<String> = to_indices.into_iter()
                    .filter_map(|i| {
                        if i < mint_keys.len() {
                            Some(mint_keys[i].clone())
                        } else {
                            None
                        }
                    })
                    .collect();

                route_map.insert(from_mint, to_mints);
            }
        }

        Ok(route_map)
    }

    // Получить цены для множества токенов
    pub async fn get_prices(&self, vs_token: Option<&str>) -> Result<HashMap<String, f64>> {
        let url = match vs_token {
            Some(token) => format!("{}/price?vsToken={}", price_api_url(), token),
            None => format!("{}/price", price_api_url()),
        };

        let response = self.http_client.get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Jupiter API error: {}", error_text));
        }

        // Парсим JSON ответ
        let price_data: HashMap<String, f64> = response.json().await
            .map_err(|e| anyhow!("Failed to parse prices response: {}", e))?;

        Ok(price_data)
    }
}