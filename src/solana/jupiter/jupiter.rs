use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use reqwest;

// Jupiter API Structures
#[derive(Debug, Serialize)]
pub struct QuoteRequest {
    pub input_mint: String,
    pub output_mint: String,
    pub amount: String,
    pub slippage_bps: u32,
    pub only_direct_routes: bool,
    pub as_legacy_transaction: bool,
}

#[derive(Debug, Deserialize)]
pub struct QuoteResponse {
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
    pub routes: Vec<Route>,
    #[serde(rename = "swapMode")]
    pub swap_mode: String,
    #[serde(rename = "slippageBps")]
    pub slippage_bps: u32,
}

#[derive(Debug, Deserialize)]
pub struct Route {
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

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct LpFee {
    pub amount: String,
    pub percent: f64,
}

#[derive(Debug, Deserialize)]
pub struct PlatformFee {
    pub amount: String,
    pub percent: f64,
}

// Jupiter Client
pub struct JupiterClient {
    http_client: reqwest::Client,
}

const JUPITER_QUOTE_API: &str = "https://quote-api.jup.ag/v6/quote";

impl JupiterClient {
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
        }
    }

    pub async fn get_quote(&self, request: QuoteRequest) -> Result<QuoteResponse> {
        let response = self.http_client.get(JUPITER_QUOTE_API)
            .query(&[
                ("inputMint", &request.input_mint),
                ("outputMint", &request.output_mint),
                ("amount", &request.amount),
                ("slippageBps", &request.slippage_bps.to_string()),
                ("onlyDirectRoutes", &request.only_direct_routes.to_string()),
                ("asLegacyTransaction", &request.as_legacy_transaction.to_string())
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Jupiter API error: {}", error_text));
        }

        let quote = response.json::<QuoteResponse>().await?;
        Ok(quote)
    }
}