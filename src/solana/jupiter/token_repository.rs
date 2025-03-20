// src/repositories/token_repository.rs
use crate::solana::jupiter::models::Token;
use crate::solana::jupiter::{JupiterToken, SOL_MINT, USDC_MINT};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use log::{error, info};
use reqwest::Client;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Repository for working with tokens
#[async_trait]
pub trait TokenRepository: Send + Sync {
    /// Get token information by its ID
    async fn get_token_by_id(&self, token_id: &str) -> Result<Token>;
}

/// Implementation of the repository for working with Jupiter tokens
pub struct JupiterTokenRepository {
    http_client: Client,
    token_cache: Arc<Mutex<HashMap<String, Token>>>,
}

impl JupiterTokenRepository {
    /// Creates a new instance of the Jupiter repository
    pub fn new() -> Self {
        Self {
            http_client: Client::new(),
            token_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl TokenRepository for JupiterTokenRepository {
    /// Gets token information by its ID
    async fn get_token_by_id(&self, token_id: &str) -> Result<Token> {
        info!("Getting token by ID: {}", token_id);

        // Check cache first
        {
            let cache = self.token_cache.lock().unwrap();
            if let Some(token) = cache.get(token_id) {
                return Ok(token.clone());
            }
        }

        // Request token via API
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
            // If it's SOL or USDC, return a placeholder
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

        // Parse the response
        let jupiter_token: JupiterToken = response.json().await.map_err(|e| {
            error!("Failed to parse token response: {}", e);
            anyhow!("Failed to parse token response: {}", e)
        })?;

        // Convert to our token format
        let token = Token {
            id: jupiter_token.address,
            symbol: jupiter_token.symbol,
            name: jupiter_token.name,
            decimals: jupiter_token.decimals,
            logo_uri: jupiter_token.logo_uri.unwrap_or_default(),
        };

        // Update cache
        {
            let mut cache = self.token_cache.lock().unwrap();
            cache.insert(token.id.clone(), token.clone());
        }

        Ok(token)
    }
}
