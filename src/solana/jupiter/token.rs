use serde::{Deserialize, Serialize};

// Константы для преобразования
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112"; // Mint адрес SOL
pub const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"; // Mint адрес USDC

// Типы токенов для работы с ними по ID вместо символов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub id: String,          // ID токена (mint адрес)
    pub symbol: String,      // Символ токена (e.g. "SOL", "USDC")
    pub name: String,        // Полное название токена
    pub decimals: u8,        // Количество десятичных знаков
    pub logo_uri: String,    // URI логотипа токена (опционально)
}

// Ответ с ценой токена
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPrice {
    pub token_id: String,           // ID токена (mint)
    pub symbol: String,             // Символ токена
    pub price_in_sol: f64,          // Цена в SOL
    pub price_in_usdc: f64,         // Цена в USDC
    pub timestamp: u64,             // Время получения цены
}