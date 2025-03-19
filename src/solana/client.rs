use anyhow::{anyhow, Result};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use std::sync::Arc;

/// Create a Solana client with confirmed commitment
pub fn create_solana_client(rpc_url: &str) -> Result<Arc<RpcClient>> {
    let client = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());

    Ok(Arc::new(client))
}
