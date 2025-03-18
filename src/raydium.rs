use crate::solana::{get_mint_from_symbol, get_symbol_from_mint};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Jupiter API URLs
const JUPITER_QUOTE_API: &str = "https://quote-api.jup.ag/v6/quote";
const RAYDIUM_PRICE_API: &str = "https://api-v3.raydium.io/mint/price"; // Оставляем для получения цен

// Структура для ответа Jupiter API
#[derive(Deserialize, Debug)]
pub struct JupiterQuote {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "otherAmountThreshold")]
    pub other_amount_threshold: String,
    pub routes: Vec<JupiterRoute>,
    #[serde(rename = "swapMode")]
    pub swap_mode: String,
    #[serde(rename = "slippageBps")]
    pub slippage_bps: u32,
    #[serde(rename = "contextSlot")]
    pub context_slot: Option<u64>,
    #[serde(rename = "timeTaken")]
    pub time_taken: Option<f64>,
    // Дополнительные поля можно добавить по мере необходимости
}

#[derive(Deserialize, Debug)]
pub struct JupiterRoute {
    #[serde(rename = "marketInfos")]
    pub market_infos: Vec<MarketInfo>,
    pub amount: String,
    #[serde(rename = "otherAmountThreshold")]
    pub other_amount_threshold: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "priceImpactPct")]
    pub price_impact_pct: f64,
    pub percent: u32,
}

#[derive(Deserialize, Debug)]
pub struct MarketInfo {
    pub id: String,
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "lpFee")]
    pub lp_fee: LpFee,
    #[serde(rename = "platformFee")]
    pub platform_fee: PlatformFee,
}

#[derive(Deserialize, Debug)]
pub struct LpFee {
    pub amount: String,
    pub percent: f64,
}

#[derive(Deserialize, Debug)]
pub struct PlatformFee {
    pub amount: String,
    pub percent: f64,
}

// Структура для Raydium API ответа (оставляем для получения цен)
#[derive(Deserialize)]
struct RaydiumResponse {
    id: String,
    success: bool,
    data: HashMap<String, String>,
}

// Get swap quote from Jupiter
pub async fn get_swap_quote(
    amount: f64,
    source_token: &str,
    target_token: &str,
    slippage: f64
) -> Result<JupiterQuote> {
    // Get mint addresses from token symbols
    let source_mint = get_mint_from_symbol(source_token)
        .ok_or_else(|| anyhow!("Unsupported source token: {}", source_token))?;

    let target_mint = get_mint_from_symbol(target_token)
        .ok_or_else(|| anyhow!("Unsupported target token: {}", target_token))?;

    // Convert amount to token units (assuming common 9 decimals - adjust as needed)
    let amount_in = (amount * 1_000_000_000.0) as u64;

    // Convert slippage to basis points (1% = 100 bps)
    let slippage_bps = (slippage * 10000.0) as u32;

    // Send request to Jupiter API
    let client = reqwest::Client::new();
    let response = client.get(JUPITER_QUOTE_API)
        .query(&[
            ("inputMint", &source_mint),
            ("outputMint", &target_mint),
            ("amount", &amount_in.to_string()),
            ("slippageBps", &slippage_bps.to_string()),
            ("onlyDirectRoutes", &"false".to_string()),
            ("asLegacyTransaction", &"false".to_string())
        ])
        .send()
        .await
        .map_err(|e| anyhow!("Failed to connect to Jupiter API: {}", e))?;

    // Check if response is successful
    if !response.status().is_success() {
        let error_text = response.text().await
            .unwrap_or_else(|_| "Unknown error".to_string());

        return Err(anyhow!("Jupiter API error: {}", error_text));
    }

    // Parse response
    let quote = response.json::<JupiterQuote>().await
        .map_err(|e| anyhow!("Failed to parse Jupiter quote response: {}", e))?;

    Ok(quote)
}

// Execute swap on Jupiter
// Note: This is a simplified placeholder. Actual implementation would involve
// creating and sending transaction with instructions specific to Jupiter
pub async fn execute_swap(
    quote: &JupiterQuote,
    keypair_base58: &str
) -> Result<String> {
    // In a real implementation, this would:
    // 1. Get the swap instruction from Jupiter's swap endpoint
    // 2. Create a transaction with the necessary Jupiter swap instructions
    // 3. Sign it with the user's keypair
    // 4. Send it to the Solana network
    // 5. Return the transaction signature

    // This is just a placeholder for the structure
    Err(anyhow!("Jupiter swap execution not implemented yet."))
}

// Get token price (оставляем использование Raydium API для цен)
pub async fn get_token_price(token_symbol: &str) -> Result<f64> {
    // Get mint address from token symbol
    let mint = get_mint_from_symbol(token_symbol)
        .ok_or_else(|| anyhow!("Unsupported token: {}", token_symbol))?;

    // Send request to Raydium price API
    let client = reqwest::Client::new();
    let response = client.get(RAYDIUM_PRICE_API)
        .query(&[("mints", &mint)])
        .send()
        .await
        .map_err(|e| anyhow!("Failed to connect to Raydium API: {}", e))?;

    // Check if response is successful
    if !response.status().is_success() {
        let error_text = response.text().await
            .unwrap_or_else(|_| "Unknown error".to_string());

        return Err(anyhow!("Raydium API error: {}", error_text));
    }

    // Parse response
    let raydium_response: RaydiumResponse = response.json().await
        .map_err(|e| anyhow!("Failed to parse Raydium price response: {}", e))?;

    // Extract prices map and convert string values to f64
    let prices = raydium_response.data.into_iter()
        .map(|(key, value)| {
            let price = value.parse::<f64>()
                .map_err(|e| anyhow!("Failed to parse Raydium price response: {}", e))?;
            Ok((key, price))
        })
        .collect::<Result<HashMap<String, f64>, anyhow::Error>>()?;

    // Get price for the requested token
    let price = prices.get(&mint)
        .ok_or_else(|| anyhow!("Price not available for token: {}", token_symbol))?
        .to_owned();

    Ok(price)
}

// Get all available token prices (оставляем использование Raydium API для цен)
pub async fn get_all_prices() -> Result<HashMap<String, f64>> {
    // Send request to Raydium price API
    let client = reqwest::Client::new();
    let response = client.get(RAYDIUM_PRICE_API)
        .send()
        .await
        .map_err(|e| anyhow!("Failed to connect to Raydium API: {}", e))?;

    // Check if response is successful
    if !response.status().is_success() {
        let error_text = response.text().await
            .unwrap_or_else(|_| "Unknown error".to_string());

        return Err(anyhow!("Raydium API error: {}", error_text));
    }

    // Parse response using the correct structure
    let raydium_response: RaydiumResponse = response.json().await
        .map_err(|e| anyhow!("Failed to parse Raydium price response: {}", e))?;

    // Extract prices map and convert string values to f64
    let mint_prices = raydium_response.data.into_iter()
        .map(|(key, value)| {
            let price = value.parse::<f64>()
                .map_err(|e| anyhow!("Failed to parse price as f64: {}", e))?;
            Ok((key, price))
        })
        .collect::<Result<HashMap<String, f64>, anyhow::Error>>()?;

    // Convert mint addresses to token symbols
    let mut symbol_prices = HashMap::new();

    for (mint, price) in mint_prices {
        let symbol = get_symbol_from_mint(&mint);
        if symbol != "Unknown" {
            symbol_prices.insert(symbol, price);
        }
    }

    Ok(symbol_prices)
}