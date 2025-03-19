use serde::{Deserialize, Serialize};
use sqlx::types::chrono::{DateTime, Utc};

// User model matching the database schema
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: i32,
    pub telegram_id: i64,
    pub username: Option<String>,
    pub solana_address: Option<String>,
    pub encrypted_private_key: Option<String>,
    pub mnemonic: Option<String>,
    pub created_at: DateTime<Utc>,
}

// Transaction model
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Transaction {
    pub id: i32,
    pub user_id: i32,
    pub recipient_address: String,
    pub amount: f64,
    pub token_symbol: String,
    pub tx_signature: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub status: String,
}

// Swap model
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Swap {
    pub id: i32,
    pub user_id: i32,
    pub from_token: String,
    pub to_token: String,
    pub amount_in: f64,
    pub amount_out: f64,
    pub tx_signature: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub status: String,
}

// Token balance for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    pub symbol: String,
    pub amount: f64,
    pub mint_address: String,
}

// Command state machine for dialogue
#[derive(Clone, Default, Debug)]
pub enum State {
    #[default]
    Start,
    AwaitingRecipientAddress,
    AwaitingAmount {
        recipient: String,
    },
    AwaitingConfirmation {
        recipient: String,
        amount: f64,
        token: String,
    },
    AwaitingSwapDetails,
}

// Swap parameters for the Raydium API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapParams {
    pub amount_in: f64,
    pub source_token: String,
    pub target_token: String,
    pub slippage: f64,
}

// Raydium quote response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumQuote {
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    pub other_amount_threshold: String,
    pub slippage_bps: u32,
    pub route: Vec<String>,
    pub platform_fee: Option<PlatformFee>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformFee {
    pub amount: String,
    pub fee_bps: u32,
}

// Error type for the application
#[derive(Debug, thiserror::Error)]
pub enum BotError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Solana client error: {0}")]
    SolanaClient(String),

    #[error("Raydium API error: {0}")]
    RaydiumApi(String),

    #[error("Telegram API error: {0}")]
    TelegramApi(#[from] teloxide::RequestError),

    #[error("Wallet not found")]
    WalletNotFound,

    #[error("Insufficient funds")]
    InsufficientFunds,

    #[error("Invalid address")]
    InvalidAddress,

    #[error("Invalid amount")]
    InvalidAmount,

    #[error("Failed to create wallet: {0}")]
    WalletCreationError(String),
}
