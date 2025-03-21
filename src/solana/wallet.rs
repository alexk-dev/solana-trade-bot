use anyhow::{anyhow, Result};
use bip39::{Language, Mnemonic};
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::{rng, RngCore};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use std::str::FromStr;

/// Generate new wallet with mnemonic phrase.
pub fn generate_wallet() -> Result<(String, String, String)> {
    // 1) Create 16 bytes (128 bits) of random entropy
    //    (enough for a 12-word BIP39 mnemonic).
    let mut entropy = [0u8; 16];
    rng().fill_bytes(&mut entropy);

    // 2) Form a 12-word mnemonic (English).
    let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
        .map_err(|e| anyhow!("Failed to create mnemonic: {}", e))?;

    // 3) Extract 64-byte seed from the mnemonic.
    //    First 32 bytes - Ed25519 private key,
    //    remaining 32 - chain code (not directly used in Solana).
    let seed = mnemonic.to_seed("");

    // 4) Create Ed25519 key, using only the first 32 bytes as seed.
    let signing_key = SigningKey::try_from(&seed[..32])
        .map_err(|e| anyhow!("Failed to create ed25519 signing key: {}", e))?;
    let verifying_key = VerifyingKey::from(&signing_key);

    // 5) Combine (32 bytes private + 32 bytes public) into one 64-byte array.
    let mut ed25519_bytes = [0u8; 64];
    ed25519_bytes[..32].copy_from_slice(&signing_key.to_bytes());
    ed25519_bytes[32..].copy_from_slice(&verifying_key.to_bytes());

    // 6) Create Solana Keypair from these 64 bytes.
    let sol_keypair = Keypair::from_bytes(&ed25519_bytes)
        .map_err(|e| anyhow!("Failed to create Solana keypair: {}", e))?;

    // 7) Get pubkey and serialize private key to base58.
    let pubkey = sol_keypair.pubkey();
    let keypair_base58 = keypair_to_base58(&sol_keypair)?;

    Ok((
        mnemonic.to_string(), // 12-word phrase
        keypair_base58,       // private key (base58)
        pubkey.to_string(),   // Solana public key
    ))
}

/// Serialize Keypair (64 bytes) to base58.
pub fn keypair_to_base58(keypair: &Keypair) -> Result<String> {
    let keypair_bytes = keypair.to_bytes();
    Ok(bs58::encode(keypair_bytes).into_string())
}

/// Restore Keypair from base58 string (64 bytes).
pub fn keypair_from_base58(keypair_base58: &str) -> Result<Keypair> {
    let keypair_bytes = bs58::decode(keypair_base58)
        .into_vec()
        .map_err(|e| anyhow!("Failed to decode base58 keypair: {}", e))?;

    if keypair_bytes.len() != 64 {
        return Err(anyhow!("Invalid keypair length: {}", keypair_bytes.len()));
    }

    let keypair = Keypair::from_bytes(&keypair_bytes)
        .map_err(|e| anyhow!("Failed to create keypair from bytes: {}", e))?;

    Ok(keypair)
}

/// Convert base58 string to Solana `Pubkey`.
pub fn parse_pubkey(address: &str) -> Result<Pubkey> {
    Pubkey::from_str(address).map_err(|e| anyhow!("Invalid Solana address: {}", e))
}
