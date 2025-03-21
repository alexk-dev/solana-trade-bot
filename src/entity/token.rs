use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub id: String,       // ID токена (mint адрес)
    pub symbol: String,   // Символ токена (e.g. "SOL", "USDC")
    pub name: String,     // Полное название токена
    pub decimals: u8,     // Количество десятичных знаков
    pub logo_uri: String, // URI логотипа токена (опционально)
}
