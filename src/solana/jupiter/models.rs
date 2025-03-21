use std::collections::HashMap;
// src/solana/jupiter/models.rs
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

// Константы для токенов
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
pub const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

// Режимы обмена (точное входное или выходное количество)
#[derive(Serialize, Deserialize, Default, PartialEq, Clone, Debug)]
pub enum SwapMode {
    #[default]
    ExactIn,
    ExactOut,
}

impl FromStr for SwapMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "ExactIn" => Ok(Self::ExactIn),
            "ExactOut" => Ok(Self::ExactOut),
            _ => Err(anyhow!("Parse SwapMode error: Invalid value '{}'", s)),
        }
    }
}

impl fmt::Display for SwapMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::ExactIn => write!(f, "ExactIn"),
            Self::ExactOut => write!(f, "ExactOut"),
        }
    }
}

// Модуль для десериализации строковых или числовых значений как float
pub mod string_or_float {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::fmt;

    pub fn serialize<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f64(*value)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<f64, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringOrFloat;

        impl<'de> serde::de::Visitor<'de> for StringOrFloat {
            type Value = f64;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a float or a string containing a float")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                value.parse::<f64>().map_err(serde::de::Error::custom)
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_str(&value)
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(value)
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(value as f64)
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(value as f64)
            }
        }

        deserializer.deserialize_any(StringOrFloat)
    }
}

#[derive(Debug, Deserialize)]
pub struct JupiterToken {
    pub address: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    #[serde(rename = "logoURI")]
    pub logo_uri: Option<String>,
}

// Структура для ответа API цен Jupiter
#[derive(Debug, Deserialize)]
pub struct JupiterPriceResponse {
    data: HashMap<String, TokenData>,
    #[serde(rename = "timeTaken")]
    time_taken: f64,
    #[serde(rename = "responseCode")]
    response_code: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenData {
    id: String,
    #[serde(rename = "type")]
    token_type: String,
    price: f64,
}

// Конфигурация для получения котировки
#[derive(Default, Debug, Clone)]
pub struct QuoteParams {
    pub input_mint: String,
    pub output_mint: String,
    pub amount: u64,
    pub slippage_bps: u64,
    pub only_direct_routes: Option<bool>,
    pub exclude_dexes: Option<Vec<String>>,
    pub max_accounts: Option<u64>,
}

// Ответ API с котировкой
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteResponse {
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    pub other_amount_threshold: String,
    pub swap_mode: String,
    pub slippage_bps: u64,
    #[serde(with = "string_or_float")]
    pub price_impact_pct: f64,
    pub route_plan: Vec<RoutePlan>,
    pub context_slot: Option<u64>,
    pub time_taken: Option<f64>,
}

// Информация о маршруте обмена
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutePlan {
    pub swap_info: SwapInfo,
    pub percent: u8,
}

// Детали обмена токенов
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapInfo {
    pub amm_key: String,
    pub label: Option<String>,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    pub fee_amount: String,
    pub fee_mint: String,
}

// Приоритизация комиссий
#[derive(Debug, Clone)]
pub enum PrioritizationFeeLamports {
    Auto,
    Exact { lamports: u64 },
}

// Запрос на выполнение свопа
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapRequest {
    #[serde(rename = "userPublicKey")]
    pub user_public_key: String,
    #[serde(rename = "wrapUnwrapSOL")]
    pub wrap_and_unwrap_sol: Option<bool>,
    #[serde(rename = "useSharedAccounts")]
    pub use_shared_accounts: Option<bool>,
    #[serde(rename = "feeAccount")]
    pub fee_account: Option<String>,
    pub prioritization_fee_lamports: PrioritizationFeeLamportsWrapper,
    #[serde(rename = "asLegacyTransaction")]
    pub as_legacy_transaction: Option<bool>,
    #[serde(rename = "useTokenLedger")]
    pub use_token_ledger: Option<bool>,
    #[serde(rename = "destinationTokenAccount")]
    pub destination_token_account: Option<String>,
    pub quote_response: QuoteResponse,
}

// Обертка для сериализации PrioritizationFeeLamports
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum PrioritizationFeeLamportsWrapper {
    Auto { auto: bool },
    Exact { lamports: u64 },
}

impl From<PrioritizationFeeLamports> for PrioritizationFeeLamportsWrapper {
    fn from(fee: PrioritizationFeeLamports) -> Self {
        match fee {
            PrioritizationFeeLamports::Auto => {
                PrioritizationFeeLamportsWrapper::Auto { auto: true }
            }
            PrioritizationFeeLamports::Exact { lamports } => {
                PrioritizationFeeLamportsWrapper::Exact { lamports }
            }
        }
    }
}

// Ответ на запрос свопа
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapResponse {
    pub swap_transaction: String,
    pub last_valid_block_height: u64,
}
