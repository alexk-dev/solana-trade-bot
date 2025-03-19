use anyhow::{anyhow, Result};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction,
    signature::{Keypair, Signer},
    transaction::Transaction as SolanaTransaction,
};

/// Execute a transaction with the provided instructions
pub async fn send_transaction(
    client: &RpcClient,
    keypair: &Keypair,
    instructions: &[Instruction],
) -> Result<String> {
    // Get recent blockhash
    let recent_blockhash = client
        .get_latest_blockhash()
        .await
        .map_err(|e| anyhow!("Failed to get recent blockhash: {}", e))?;

    // Create transaction
    let transaction = SolanaTransaction::new_signed_with_payer(
        instructions,
        Some(&keypair.pubkey()),
        &[keypair],
        recent_blockhash,
    );

    // Send transaction
    let signature = client
        .send_and_confirm_transaction(&transaction)
        .await
        .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;

    Ok(signature.to_string())
}
