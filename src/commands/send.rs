use anyhow::Result;
use log::info;
use std::sync::Arc;
use teloxide::prelude::*;

use super::{CommandHandler, MyDialogue};
use crate::di::ServiceContainer;
use crate::entity::State;
use crate::interactor::send_interactor::{SendInteractor, SendInteractorImpl};
use crate::presenter::send_presenter::{SendPresenter, SendPresenterImpl};
use crate::view::send_view::TelegramSendView;

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
        services: Arc<ServiceContainer>,
    ) -> Result<()> {
        let dialogue = dialogue.ok_or_else(|| anyhow::anyhow!("Dialogue context not provided"))?;
        let chat_id = msg.chat.id;

        info!("Send command initiated");

        let db_pool = services.db_pool();
        let solana_client = services.solana_client();
        let interactor = Arc::new(SendInteractorImpl::new(db_pool, solana_client));
        let view = Arc::new(TelegramSendView::new(bot, chat_id));
        let presenter = SendPresenterImpl::new(interactor, view);

        // Start the send flow
        dialogue.update(State::AwaitingRecipientAddress).await?;
        presenter.start_send_flow().await?;

        Ok(())
    }
}

// Handler for the recipient address state
pub async fn receive_recipient_address(
    bot: Bot,
    msg: Message,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    if let Some(address_text) = msg.text() {
        let chat_id = msg.chat.id;

        let db_pool = services.db_pool();
        let solana_client = services.solana_client();
        let interactor = Arc::new(SendInteractorImpl::new(db_pool, solana_client));
        let view = Arc::new(TelegramSendView::new(bot.clone(), chat_id));
        let presenter = SendPresenterImpl::new(interactor, view);

        // Handle the recipient address
        if let Ok(is_valid) = presenter.validate_address(address_text).await {
            if is_valid {
                dialogue
                    .update(State::AwaitingAmount {
                        recipient: address_text.to_string(),
                    })
                    .await?;
                bot.send_message(
                    chat_id,
                    "Enter the amount to send (example: 0.5 SOL or 100 USDC):",
                )
                .await?;
            } else {
                bot.send_message(
                    chat_id,
                    "Invalid Solana address. Please check the address and try again:",
                )
                .await?;
            }
        } else {
            bot.send_message(chat_id, "Error validating address. Please try again:")
                .await?;
        }
    } else {
        bot.send_message(msg.chat.id, "Please enter the recipient's address as text:")
            .await?;
    }

    Ok(())
}

// Handler for the amount state
pub async fn receive_amount(
    bot: Bot,
    msg: Message,
    state: State,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    if let State::AwaitingAmount { recipient } = state {
        if let Some(amount_text) = msg.text() {
            let chat_id = msg.chat.id;

            let db_pool = services.db_pool();
            let solana_client = services.solana_client();
            let interactor = Arc::new(SendInteractorImpl::new(db_pool, solana_client));

            // Handle the amount
            match interactor.parse_amount_and_token(amount_text).await {
                Ok((amount, token)) => {
                    dialogue
                        .update(State::AwaitingConfirmation {
                            recipient: recipient.clone(),
                            amount,
                            token: token.clone(),
                        })
                        .await?;

                    bot.send_message(
                        chat_id,
                        format!(
                            "Confirm sending {} {} to address {} (yes/no):",
                            amount, token, recipient
                        ),
                    )
                    .await?;
                }
                Err(e) => {
                    bot.send_message(
                        chat_id,
                        format!("Invalid amount format: {}. Please enter in the format '0.5 SOL' or '100 USDC':", e),
                    ).await?;
                }
            }
        } else {
            bot.send_message(msg.chat.id, "Please enter the amount to send:")
                .await?;
        }
    }

    Ok(())
}

// Handler for the confirmation state
pub async fn receive_confirmation(
    bot: Bot,
    msg: Message,
    state: State,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    if let State::AwaitingConfirmation {
        recipient,
        amount,
        token,
    } = state
    {
        if let Some(text) = msg.text() {
            let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);
            let chat_id = msg.chat.id;

            // Reset dialogue state
            dialogue.update(State::Start).await?;

            // Create VIPER components
            let interactor = Arc::new(SendInteractorImpl::new(
                services.db_pool(),
                services.solana_client(),
            ));
            let view = Arc::new(TelegramSendView::new(bot.clone(), chat_id));
            let presenter = SendPresenterImpl::new(interactor, view);

            // Handle the confirmation
            presenter
                .handle_confirmation(text, &recipient, amount, &token, telegram_id)
                .await?;
        }
    }

    Ok(())
}
