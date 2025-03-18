use solana_sdk::pubkey::Pubkey;
use crate::solana::tokens::constants::{USDC_MINT, USDT_MINT, RAY_MINT};

// Constants for conversion
pub const LAMPORTS_PER_SOL: f64 = 1_000_000_000.0;

/// Convert lamports to SOL
pub fn lamports_to_sol(lamports: u64) -> f64 {
    lamports as f64 / LAMPORTS_PER_SOL
}

/// Convert SOL to lamports
pub fn sol_to_lamports(sol: f64) -> u64 {
    (sol * LAMPORTS_PER_SOL) as u64
}

/// Convert amount with decimals to token units
pub fn convert_to_token_amount(amount: f64, decimals: u8) -> u64 {
    (amount * 10_f64.powi(decimals as i32)) as u64
}

/// Get token info from mint address
pub fn get_token_info_from_mint(mint_address: Pubkey) -> (&'static str, String) {
    match mint_address.to_string().as_str() {
        USDC_MINT => ("USDC", USDC_MINT.to_string()),
        USDT_MINT => ("USDT", USDT_MINT.to_string()),
        RAY_MINT => ("RAY", RAY_MINT.to_string()),
        _ => ("Unknown", mint_address.to_string()),
    }
}

/// Get mint address from token symbol
pub fn get_mint_from_symbol(symbol: &str) -> Option<String> {
    match symbol.to_uppercase().as_str() {
        "SOL" => None, // SOL is native, not an SPL token
        "USDC" => Some(USDC_MINT.to_string()),
        "USDT" => Some(USDT_MINT.to_string()),
        "RAY" => Some(RAY_MINT.to_string()),
        _ => None,
    }
}

/// Get token symbol from mint address
pub fn get_symbol_from_mint(mint: &str) -> String {
    match mint {
        USDC_MINT => "USDC".to_string(),
        USDT_MINT => "USDT".to_string(),
        RAY_MINT => "RAY".to_string(),
        _ => "Unknown".to_string(),
    }
}