use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use log::debug;
use qrcode::{render::svg, QrCode};
use regex::Regex;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

// Generate QR code for a Solana address
pub fn generate_qr_code(address: &str) -> Result<Vec<u8>> {
    // Create QR code with high error correction
    let code = QrCode::with_error_correction_level(address, qrcode::EcLevel::H)
        .map_err(|e| anyhow!("Failed to generate QR code: {}", e))?;

    // Render QR code as SVG with modern API
    let svg_string = code
        .render()
        .min_dimensions(200, 200)
        .dark_color(svg::Color("#000000"))
        .light_color(svg::Color("#ffffff"))
        .build();

    // Convert SVG to bytes
    let svg = svg_string.into_bytes();

    Ok(svg)
}

// Validate Solana address
pub fn validate_solana_address(address: &str) -> bool {
    Pubkey::from_str(address).is_ok()
}

// Parse amount and token from input string
pub fn parse_amount_and_token(input: &str) -> Option<(f64, &str)> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^(\d+(?:\.\d+)?)\s+([A-Za-z]+)$").unwrap();
    }

    RE.captures(input).and_then(|cap| {
        let amount_str = cap.get(1)?.as_str();
        let token = cap.get(2)?.as_str();

        amount_str.parse::<f64>().ok().map(|amount| (amount, token))
    })
}

// Format amount with appropriate precision
pub fn format_amount(amount: f64, token: &str) -> String {
    match token.to_uppercase().as_str() {
        "SOL" => format!("{:.9}", amount),           // 9 decimals
        "USDC" | "USDT" => format!("{:.6}", amount), // 6 decimals
        _ => format!("{:.6}", amount),               // Default to 6 decimals
    }
}

// Validate and normalize swap parameters
pub fn validate_swap_params(
    amount: f64,
    source_token: &str,
    target_token: &str,
    slippage_percent: Option<f64>,
) -> Result<(f64, String, String, f64)> {
    // Validate amount
    if amount <= 0.0 {
        return Err(anyhow!("Amount must be greater than zero"));
    }

    // Validate tokens
    let supported_tokens = ["SOL", "USDC", "USDT", "RAY"];

    if !supported_tokens.contains(&source_token) {
        return Err(anyhow!("Unsupported source token: {}", source_token));
    }

    if !supported_tokens.contains(&target_token) {
        return Err(anyhow!("Unsupported target token: {}", target_token));
    }

    if source_token == target_token {
        return Err(anyhow!("Source and target tokens must be different"));
    }

    // Normalize slippage (default 0.5%)
    let slippage = slippage_percent.unwrap_or(0.5).max(0.1).min(5.0) / 100.0;

    Ok((
        amount,
        source_token.to_string(),
        target_token.to_string(),
        slippage,
    ))
}

// Parse Solana address and convert to pubkey
pub fn parse_solana_address(address: &str) -> Result<Pubkey> {
    Pubkey::from_str(address).map_err(|_| anyhow!("Invalid Solana address format"))
}

// Shorten address for display
pub fn shorten_address(address: &str) -> String {
    if address.len() <= 10 {
        return address.to_string();
    }

    let start = &address[..5];
    let end = &address[address.len() - 5..];

    format!("{}...{}", start, end)
}
