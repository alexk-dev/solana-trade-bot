use anyhow::Result;
use log::{error, info};
use solana_client::nonblocking::rpc_client::RpcClient;
use sqlx::PgPool;
use std::sync::Arc;
use teloxide::prelude::*;

use super::CommandHandler;
use crate::di::ServiceContainer;
use crate::model::State;
use crate::MyDialogue;
use crate::{db, solana, utils};

pub struct SendCommand;

impl CommandHandler for SendCommand {
    fn command_name() -> &'static str {
        "send"
    }

    fn description() -> &'static str {
        "send funds to another address"
    }

    async fn execute(
        bot: Bot,
        msg: Message,
        dialogue: Option<MyDialogue>,
        _solana_client: Option<Arc<RpcClient>>,
        _services: Arc<ServiceContainer>,
    ) -> Result<()> {
        let dialogue = dialogue.ok_or_else(|| anyhow::anyhow!("Dialogue context not provided"))?;
        info!("Send command initiated");

        dialogue.update(State::AwaitingRecipientAddress).await?;
        bot.send_message(msg.chat.id, "Enter the recipient's Solana address:")
            .await?;

        Ok(())
    }
}

pub async fn receive_recipient_address(bot: Bot, msg: Message, dialogue: MyDialogue) -> Result<()> {
    if let Some(address_text) = msg.text() {
        // Validate the address format
        if utils::validate_solana_address(address_text) {
            dialogue
                .update(State::AwaitingAmount {
                    recipient: address_text.to_string(),
                })
                .await?;

            bot.send_message(
                msg.chat.id,
                "Enter the amount to send (example: 0.5 SOL or 100 USDC):",
            )
            .await?;
        } else {
            bot.send_message(
                msg.chat.id,
                "Invalid Solana address. Please check the address and try again:",
            )
            .await?;
        }
    } else {
        bot.send_message(msg.chat.id, "Please enter the recipient's address as text:")
            .await?;
    }

    Ok(())
}

pub async fn receive_amount(
    bot: Bot,
    msg: Message,
    state: State,
    dialogue: MyDialogue,
) -> Result<()> {
    if let State::AwaitingAmount { recipient } = state {
        if let Some(amount_text) = msg.text() {
            // Parse amount and token from the input
            if let Some((amount, token)) = utils::parse_amount_and_token(amount_text) {
                dialogue
                    .update(State::AwaitingConfirmation {
                        recipient: recipient.clone(),
                        amount,
                        token: token.to_string(),
                    })
                    .await?;

                bot.send_message(
                    msg.chat.id,
                    format!(
                        "Confirm sending {} {} to address {} (yes/no):",
                        amount, token, recipient
                    ),
                )
                .await?;
            } else {
                bot.send_message(
                    msg.chat.id,
                    "Invalid amount format. Please enter in the format '0.5 SOL' or '100 USDC':",
                )
                .await?;
            }
        } else {
            bot.send_message(msg.chat.id, "Please enter the amount to send:")
                .await?;
        }
    }

    Ok(())
}

pub async fn receive_confirmation(
    bot: Bot,
    msg: Message,
    state: State,
    dialogue: MyDialogue,
    db_pool: PgPool,
    solana_client: Arc<RpcClient>,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    if let State::AwaitingConfirmation {
        recipient,
        amount,
        token,
    } = state
    {
        if let Some(text) = msg.text() {
            let confirmation = text.to_lowercase();

            if confirmation == "yes" {
                let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);

                // Reset dialogue state
                dialogue.update(State::Start).await?;

                // Send "processing" message
                let processing_msg = bot
                    .send_message(msg.chat.id, "Sending funds... Please wait.")
                    .await?;

                // We can use either directly passed parameters or get them from services container
                let db_pool = services.db_pool();
                let solana_client = services.solana_client();

                // Get user wallet info
                let user = db::get_user_by_telegram_id(&db_pool, telegram_id).await?;

                match user.solana_address {
                    Some(sender_address) => {
                        // Get private key
                        if let Some(keypair_base58) = user.encrypted_private_key {
                            let keypair = solana::keypair_from_base58(&keypair_base58)?;

                            // Send transaction
                            let result = if token.to_uppercase() == "SOL" {
                                solana::send_sol(&solana_client, &keypair, &recipient, amount).await
                            } else {
                                solana::send_spl_token(
                                    &solana_client,
                                    &keypair,
                                    &recipient,
                                    &token,
                                    amount,
                                )
                                .await
                            };

                            match result {
                                Ok(signature) => {
                                    // Record transaction to database
                                    db::record_transaction(
                                        &db_pool,
                                        telegram_id,
                                        &recipient,
                                        amount,
                                        &token,
                                        &Some(signature.clone()),
                                        "SUCCESS",
                                    )
                                    .await?;

                                    // Send success message
                                    bot.edit_message_text(
                                        msg.chat.id,
                                        processing_msg.id,
                                        format!(
                                            "✅ Funds sent successfully. Tx Signature: {}",
                                            signature
                                        ),
                                    )
                                    .await?;
                                }
                                Err(e) => {
                                    error!("Failed to send transaction: {}", e);

                                    // Record failed transaction
                                    db::record_transaction(
                                        &db_pool,
                                        telegram_id,
                                        &recipient,
                                        amount,
                                        &token,
                                        &None::<String>,
                                        "FAILED",
                                    )
                                    .await?;

                                    // Send error message
                                    bot.edit_message_text(
                                        msg.chat.id,
                                        processing_msg.id,
                                        format!("❌ Error sending funds: {}", e),
                                    )
                                    .await?;
                                }
                            }
                        } else {
                            bot.edit_message_text(
                                msg.chat.id,
                                processing_msg.id,
                                "❌ Error: Private key not found for your wallet.",
                            )
                            .await?;
                        }
                    }
                    None => {
                        bot.edit_message_text(
                            msg.chat.id,
                            processing_msg.id,
                            "❌ You don't have a wallet yet. Use /create_wallet to create a new wallet."
                        ).await?;
                    }
                }
            } else {
                // Transaction cancelled
                dialogue.update(State::Start).await?;

                bot.send_message(msg.chat.id, "Transaction cancelled.")
                    .await?;
            }
        }
    }

    Ok(())
}
