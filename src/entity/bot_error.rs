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
