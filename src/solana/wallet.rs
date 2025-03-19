use anyhow::{anyhow, Result};
use bip39::{Language, Mnemonic};
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::{rng, RngCore}; // Если 'rng()' не доступна – замените на {thread_rng, RngCore}.
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use std::convert::TryInto;
use std::str::FromStr;

/// Генерация нового кошелька с мнемонической фразой.
pub fn generate_wallet() -> Result<(String, String, String)> {
    // 1) Создаём 16 байт (128 бит) случайной энтропии
    //    (этого хватает на 12-словный BIP39-мнемоник).
    let mut entropy = [0u8; 16];
    rng().fill_bytes(&mut entropy);
    // Если 'rng()' не работает, используйте:
    // let mut rng = thread_rng();
    // rng.fill_bytes(&mut entropy);

    // 2) Формируем 12-словный мнемоник (English).
    let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
        .map_err(|e| anyhow!("Failed to create mnemonic: {}", e))?;

    // 3) Извлекаем 64-байтовый seed из мнемоника.
    //    Первые 32 байта — приватный ключ Ed25519,
    //    остальные 32 — chain code (в Solana не используется напрямую).
    let seed = mnemonic.to_seed("");

    // 4) Создаём Ed25519-ключ, используя только первые 32 байта в качестве seed.
    let signing_key = SigningKey::try_from(&seed[..32])
        .map_err(|e| anyhow!("Failed to create ed25519 signing key: {}", e))?;
    let verifying_key = VerifyingKey::from(&signing_key);

    // 5) Склеиваем (32 байта приватного + 32 байта публичного) в один 64-байтовый массив.
    let mut ed25519_bytes = [0u8; 64];
    ed25519_bytes[..32].copy_from_slice(&signing_key.to_bytes());
    ed25519_bytes[32..].copy_from_slice(&verifying_key.to_bytes());

    // 6) Создаём Solana Keypair из этих 64 байт.
    let sol_keypair = Keypair::from_bytes(&ed25519_bytes)
        .map_err(|e| anyhow!("Failed to create Solana keypair: {}", e))?;

    // 7) Получаем pubkey и сериализуем приватник в base58.
    let pubkey = sol_keypair.pubkey();
    let keypair_base58 = keypair_to_base58(&sol_keypair)?;

    Ok((
        mnemonic.to_string(), // 12-словная фраза
        keypair_base58,       // приватник (base58)
        pubkey.to_string(),   // публичный ключ Solana
    ))
}

/// Сериализовать Keypair (64 байта) в base58.
pub fn keypair_to_base58(keypair: &Keypair) -> Result<String> {
    let keypair_bytes = keypair.to_bytes();
    Ok(bs58::encode(keypair_bytes).into_string())
}

/// Восстановить Keypair из base58-строки (64 байта).
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

/// Преобразовать base58-строку в Solana `Pubkey`.
pub fn parse_pubkey(address: &str) -> Result<Pubkey> {
    Pubkey::from_str(address).map_err(|e| anyhow!("Invalid Solana address: {}", e))
}
