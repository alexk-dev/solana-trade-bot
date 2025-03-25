use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub id: String,       // Token ID (mint address)
    pub symbol: String,   // Token symbol (e.g. "SOL", "USDC")
    pub name: String,     // Full token name
    pub decimals: u8,     // Number of decimal places
    pub logo_uri: String, // Token logo URI (optional)
}
