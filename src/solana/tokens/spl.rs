use anyhow::{anyhow, Result};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_request::TokenAccountsFilter;
use solana_client::rpc_response::RpcKeyedAccount;
use solana_sdk::account::Account;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account_idempotent,
};
use spl_token::{instruction as token_instruction, ID as TOKEN_PROGRAM_ID};

use crate::model::{BotError, TokenBalance};
use crate::solana::jupiter::token_repository::JupiterTokenRepository;
use crate::solana::jupiter::TokenRepository;
use crate::solana::tokens::constants::{RAY_MINT, USDC_MINT, USDT_MINT};
use crate::solana::tokens::transaction::send_transaction;
use crate::solana::utils::convert_to_token_amount;
use crate::solana::wallet::parse_pubkey;

/// Get token balances
pub async fn get_token_balances(client: &RpcClient, address: &str) -> Result<Vec<TokenBalance>> {
    let pubkey: Pubkey = parse_pubkey(address)?;

    // 1) The list of token accounts is returned as UiAccount
    let token_accounts: Vec<RpcKeyedAccount> = client
        .get_token_accounts_by_owner(&pubkey, TokenAccountsFilter::ProgramId(spl_token::ID))
        .await
        .map_err(|e| anyhow!("Failed to get token accounts: {}", e))?;

    let mut balances: Vec<TokenBalance> = Vec::new();

    for keyed_account in token_accounts {
        let token_account_pubkey: Pubkey = parse_pubkey(&keyed_account.pubkey.to_string())?;
        //
        // let token_account = client.get_account(&token_account_pubkey).await?;

        // let balance = client
        //     .get_token_account_balance(&token_account_pubkey)
        //     .await
        //     .unwrap();

        let token_account = client
            .get_token_account(&token_account_pubkey)
            .await?
            .unwrap();
        let mint_id = token_account.mint.to_string();
        let token_amount = token_account.token_amount.ui_amount.unwrap();

        let token_repository = JupiterTokenRepository::new();
        let token = token_repository
            .get_token_by_id(&mint_id)
            .await
            .map_err(|e| anyhow!("Failed to get token: {}", e))?;

        balances.push(TokenBalance {
            symbol: token.symbol,
            amount: token_amount,
            mint_address: mint_id.clone(),
        });
    }

    Ok(balances)
}

/// Send SPL token
pub async fn send_spl_token(
    client: &RpcClient,
    keypair: &Keypair,
    recipient: &str,
    token_symbol: &str,
    amount: f64,
) -> Result<String> {
    // Convert recipient string to pubkey
    let recipient_pubkey: Pubkey = parse_pubkey(recipient)?;

    // Get token mint address based on symbol
    let mint_address: &str = match token_symbol.to_uppercase().as_str() {
        "USDC" => USDC_MINT,
        "USDT" => USDT_MINT,
        "RAY" => RAY_MINT,
        _ => return Err(anyhow!("Unsupported token symbol: {}", token_symbol)),
    };

    let mint_pubkey: Pubkey = parse_pubkey(mint_address)?;

    // Get sender's token account
    let sender_pubkey: Pubkey = keypair.pubkey();
    let sender_token_account: Pubkey = get_associated_token_address(&sender_pubkey, &mint_pubkey);

    // Check if sender has the token account
    match client.get_account(&sender_token_account).await {
        Ok(sender_token_account_info) => {
            // sender_token_account_info has Account type (raw).
            let account_data: Vec<u8> = sender_token_account_info.data;

            if account_data.len() < 72 {
                return Err(anyhow!("Sender token account data too short").into());
            }

            let token_account_amount: u64 = u64::from_le_bytes(account_data[64..72].try_into()?);

            // Get mint info
            let mint_info: Account = client
                .get_account(&mint_pubkey)
                .await
                .map_err(|e| anyhow!("Failed to get mint info: {}", e))?;

            // mint_info.data is also Vec<u8>
            let mint_data: Vec<u8> = mint_info.data;

            let decimals: u8 = if mint_data.len() > 44 {
                mint_data[44]
            } else {
                6
            };

            // Convert amount to token units
            let token_amount: u64 = convert_to_token_amount(amount, decimals);

            // Make sure sender has enough tokens
            if token_account_amount < token_amount {
                return Err(BotError::InsufficientFunds.into());
            }

            // Get or create recipient's associated token account
            let recipient_token_account: Pubkey =
                get_associated_token_address(&recipient_pubkey, &mint_pubkey);

            // Prepare instructions
            let mut instructions = Vec::new();

            // Check if recipient token account exists and create if not
            if client.get_account(&recipient_token_account).await.is_err() {
                instructions.push(create_associated_token_account_idempotent(
                    &sender_pubkey,
                    &recipient_pubkey,
                    &mint_pubkey,
                    &TOKEN_PROGRAM_ID,
                ));
            }

            // Add token transfer instruction
            instructions.push(
                token_instruction::transfer(
                    &TOKEN_PROGRAM_ID,
                    &sender_token_account,
                    &recipient_token_account,
                    &sender_pubkey,
                    &[&sender_pubkey],
                    token_amount,
                )
                .map_err(|e| anyhow!("Failed to create token transfer instruction: {}", e))?,
            );

            // Execute transaction
            send_transaction(client, keypair, &instructions).await
        }
        Err(_) => Err(anyhow!(
            "Sender doesn't have a token account for {}",
            token_symbol
        )),
    }
}

/// Get balance of a specific SPL token
pub async fn get_spl_token_balance(
    client: &RpcClient,
    address: &str,
    token_symbol: &str,
) -> Result<f64> {
    let balances: Vec<TokenBalance> = get_token_balances(client, address).await?;

    for balance in balances {
        if balance.symbol.to_uppercase() == token_symbol.to_uppercase() {
            return Ok(balance.amount);
        }
    }
    // If token not found, return 0
    Ok(0.0)
}
