use anyhow::{anyhow, Result};
use async_trait::async_trait;
use log::debug;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

use crate::solana::jupiter::Config;

/// Interface for exchange route service
#[async_trait]
pub trait RouteService: Send + Sync {
    /// Get available exchange routes between tokens
    async fn get_route_map(&self) -> Result<HashMap<String, Vec<String>>>;
}

/// Implementation of route service using Jupiter API
pub struct JupiterRouteService {
    http_client: Client,
    config: Config,
}

impl JupiterRouteService {
    /// Creates a new instance of route service
    pub fn new(config: Config) -> Self {
        Self {
            http_client: Client::new(),
            config,
        }
    }
}

#[async_trait]
impl RouteService for JupiterRouteService {
    /// Get available routes map
    async fn get_route_map(&self) -> Result<HashMap<String, Vec<String>>> {
        let url = format!(
            "{}/indexed-route-map?onlyDirectRoutes=false",
            self.config.quote_api_url
        );

        debug!("Requesting route map from: {}", url);

        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct IndexedRouteMap {
            mint_keys: Vec<String>,
            indexed_route_map: HashMap<usize, Vec<usize>>,
        }

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

        let route_map_response = response
            .json::<IndexedRouteMap>()
            .await
            .map_err(|e| anyhow!("Failed to parse route map response: {}", e))?;

        let mint_keys = route_map_response.mint_keys;
        let mut route_map = HashMap::new();

        for (from_index, to_indices) in route_map_response.indexed_route_map {
            if from_index < mint_keys.len() {
                let from_mint = mint_keys[from_index].clone();
                let to_mints: Vec<String> = to_indices
                    .into_iter()
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
}
