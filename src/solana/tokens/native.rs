use crate::entity::BotError;
use crate::solana::tokens::constants::ESTIMATED_SOL_FEE;
use crate::solana::tokens::transaction::send_transaction;
use crate::solana::utils::{lamports_to_sol, sol_to_lamports};
use crate::solana::wallet::parse_pubkey;
use anyhow::{anyhow, Result};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    signature::{Keypair, Signer},
    system_instruction,
};

/// Get SOL balance
pub async fn get_sol_balance(client: &RpcClient, address: &str) -> Result<f64> {
    let pubkey = parse_pubkey(address)?;

    let balance = client
        .get_balance(&pubkey)
        .await
        .map_err(|e| anyhow!("Failed to get balance: {}", e))?;

    // Convert from lamports to SOL
    Ok(lamports_to_sol(balance))
}

/// Send SOL
pub async fn send_sol(
    client: &RpcClient,
    keypair: &Keypair,
    recipient: &str,
    amount: f64,
) -> Result<String> {
    // Convert recipient string to pubkey
    let recipient_pubkey = parse_pubkey(recipient)?;

    // Check sender balance
    let sender_pubkey = keypair.pubkey();
    let sender_balance = client
        .get_balance(&sender_pubkey)
        .await
        .map_err(|e| anyhow!("Failed to get sender balance: {}", e))?;

    // Convert amount to lamports
    let lamports = sol_to_lamports(amount);

    // Make sure sender has enough balance (including estimated fees)
    if sender_balance < lamports + ESTIMATED_SOL_FEE {
        return Err(BotError::InsufficientFunds.into());
    }

    // Create transfer instruction
    let instruction = system_instruction::transfer(&sender_pubkey, &recipient_pubkey, lamports);

    // Execute transaction
    send_transaction(client, keypair, &[instruction]).await
}
