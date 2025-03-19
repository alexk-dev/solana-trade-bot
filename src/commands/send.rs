// src/commands/send.rs
use anyhow::Result;
use log::{info, error};
use sqlx::PgPool;
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use teloxide::prelude::*;

use crate::{db, solana, utils};
use crate::model::State;
use crate::MyDialogue;
use super::CommandHandler;

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
        _db_pool: Option<PgPool>,
        _solana_client: Option<Arc<RpcClient>>
    ) -> Result<()> {
        let dialogue = dialogue.ok_or_else(|| anyhow::anyhow!("Dialogue context not provided"))?;
        info!("Send command initiated");

        dialogue.update(State::AwaitingRecipientAddress).await?;
        bot.send_message(msg.chat.id, "Введите Solana-адрес получателя:").await?;

        Ok(())
    }
}

pub async fn receive_recipient_address(bot: Bot, msg: Message, dialogue: MyDialogue) -> Result<()> {
    if let Some(address_text) = msg.text() {
        // Validate the address format
        if utils::validate_solana_address(address_text) {
            dialogue.update(State::AwaitingAmount { recipient: address_text.to_string() }).await?;

            bot.send_message(
                msg.chat.id,
                "Введите сумму для отправки (например: 0.5 SOL или 100 USDC):"
            ).await?;
        } else {
            bot.send_message(
                msg.chat.id,
                "Некорректный Solana-адрес. Пожалуйста, проверьте адрес и попробуйте снова:"
            ).await?;
        }
    } else {
        bot.send_message(
            msg.chat.id,
            "Пожалуйста, введите текстовый адрес получателя:"
        ).await?;
    }

    Ok(())
}

pub async fn receive_amount(
    bot: Bot,
    msg: Message,
    state: State,
    dialogue: MyDialogue
) -> Result<()> {
    if let State::AwaitingAmount { recipient } = state {
        if let Some(amount_text) = msg.text() {
            // Parse amount and token from the input
            if let Some((amount, token)) = utils::parse_amount_and_token(amount_text) {
                dialogue.update(State::AwaitingConfirmation {
                    recipient: recipient.clone(),
                    amount,
                    token: token.to_string()
                }).await?;

                bot.send_message(
                    msg.chat.id,
                    format!(
                        "Подтвердите отправку {} {} на адрес {} (да/нет):",
                        amount, token, recipient
                    )
                ).await?;
            } else {
                bot.send_message(
                    msg.chat.id,
                    "Некорректный формат суммы. Пожалуйста, введите в формате '0.5 SOL' или '100 USDC':"
                ).await?;
            }
        } else {
            bot.send_message(
                msg.chat.id,
                "Пожалуйста, введите сумму для отправки:"
            ).await?;
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
    solana_client: Arc<RpcClient>
) -> Result<()> {
    if let State::AwaitingConfirmation { recipient, amount, token } = state {
        if let Some(text) = msg.text() {
            let confirmation = text.to_lowercase();

            if confirmation == "да" || confirmation == "yes" {
                let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);

                // Reset dialogue state
                dialogue.update(State::Start).await?;

                // Send "processing" message
                let processing_msg = bot.send_message(
                    msg.chat.id,
                    "Отправка средств... Пожалуйста, подождите."
                ).await?;

                // Get user wallet info
                let user = db::get_user_by_telegram_id(&db_pool, telegram_id).await?;

                match user.solana_address {
                    Some(sender_address) => {
                        // Get private key
                        if let Some(keypair_base58) = user.encrypted_private_key {
                            let keypair = solana::keypair_from_base58(&keypair_base58)?;

                            // Send transaction
                            let result = if token.to_uppercase() == "SOL" {
                                solana::send_sol(
                                    &solana_client,
                                    &keypair,
                                    &recipient,
                                    amount
                                ).await
                            } else {
                                solana::send_spl_token(
                                    &solana_client,
                                    &keypair,
                                    &recipient,
                                    &token,
                                    amount
                                ).await
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
                                        "SUCCESS"
                                    ).await?;

                                    // Send success message
                                    bot.edit_message_text(
                                        msg.chat.id,
                                        processing_msg.id,
                                        format!("✅ Средства отправлены. Tx Signature: {}", signature)
                                    ).await?;
                                },
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
                                        "FAILED"
                                    ).await?;

                                    // Send error message
                                    bot.edit_message_text(
                                        msg.chat.id,
                                        processing_msg.id,
                                        format!("❌ Ошибка при отправке средств: {}", e)
                                    ).await?;
                                }
                            }
                        } else {
                            bot.edit_message_text(
                                msg.chat.id,
                                processing_msg.id,
                                "❌ Ошибка: Не найден закрытый ключ для вашего кошелька."
                            ).await?;
                        }
                    },
                    None => {
                        bot.edit_message_text(
                            msg.chat.id,
                            processing_msg.id,
                            "❌ У вас еще нет кошелька. Используйте /create_wallet чтобы создать новый кошелек."
                        ).await?;
                    }
                }
            } else {
                // Transaction cancelled
                dialogue.update(State::Start).await?;

                bot.send_message(
                    msg.chat.id,
                    "Отправка средств отменена."
                ).await?;
            }
        }
    }

    Ok(())
}