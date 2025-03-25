use anyhow::Result;
use log::info;
use std::sync::Arc;
use teloxide::prelude::*;

use super::{CommandHandler, MyDialogue};
use crate::di::ServiceContainer;
use crate::entity::State;
use crate::interactor::withdraw_interactor::{WithdrawInteractor, WithdrawInteractorImpl};
use crate::presenter::withdraw_presenter::{WithdrawPresenter, WithdrawPresenterImpl};
use crate::view::withdraw_view::TelegramWithdrawView;

pub struct WithdrawCommand;

impl CommandHandler for WithdrawCommand {
    fn command_name() -> &'static str {
        "withdraw"
    }

    fn description() -> &'static str {
        "withdraw tokens to another address"
    }

    async fn execute(
        bot: Bot,
        msg: Message,
        telegram_id: i64,
        dialogue: Option<MyDialogue>,
        services: Arc<ServiceContainer>,
    ) -> Result<()> {
        let dialogue = dialogue.ok_or_else(|| anyhow::anyhow!("Dialogue context not provided"))?;
        let chat_id = msg.chat.id;

        info!("Withdraw command initiated by user: {}", telegram_id);

        // Update dialogue state
        dialogue
            .update(State::AwaitingWithdrawTokenSelection)
            .await?;

        // Create presenter
        let db_pool = services.db_pool();
        let solana_client = services.solana_client();
        let price_service = services.price_service();

        let interactor = Arc::new(WithdrawInteractorImpl::new(
            db_pool,
            solana_client,
            price_service,
        ));
        let view = Arc::new(TelegramWithdrawView::new(bot, chat_id));
        let presenter = WithdrawPresenterImpl::new(interactor, view);

        // Start the withdraw flow
        presenter.start_withdraw_flow(telegram_id).await?;

        Ok(())
    }
}

// Handler for recipient address state
pub async fn receive_recipient_address(
    bot: Bot,
    msg: Message,
    state: State,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    if let State::AwaitingWithdrawRecipientAddress {
        token_address,
        token_symbol,
        amount,
        price_in_sol,
        price_in_usdc,
    } = state
    {
        if let Some(address_text) = msg.text() {
            let chat_id = msg.chat.id;
            let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);

            // Create presenter
            let db_pool = services.db_pool();
            let solana_client = services.solana_client();
            let price_service = services.price_service();

            let interactor = Arc::new(WithdrawInteractorImpl::new(
                db_pool,
                solana_client,
                price_service,
            ));
            let view = Arc::new(TelegramWithdrawView::new(bot.clone(), chat_id));
            let presenter = WithdrawPresenterImpl::new(interactor.clone(), view);

            // Check if address is valid
            if let Ok(is_valid) = interactor.validate_recipient_address(address_text).await {
                if is_valid {
                    // Update dialogue state
                    dialogue
                        .update(State::AwaitingWithdrawAmount {
                            token_address: token_address.clone(),
                            token_symbol: token_symbol.clone(),
                            recipient: address_text.to_string(),
                            balance: amount,
                            price_in_sol,
                            price_in_usdc,
                        })
                        .await?;

                    // Prompt for amount
                    bot.send_message(
                        chat_id,
                        format!(
                            "You have <b>{:.6} {}</b> (worth {:.6} SOL / ${:.2}).\n\n\
                            Enter the amount to withdraw:\n\
                            • Enter a specific amount (e.g. <code>0.5</code>)\n\
                            • Enter a percentage (e.g. <code>50%</code>)\n\
                            • Or type <code>All</code> to withdraw your entire balance",
                            amount,
                            token_symbol,
                            amount * price_in_sol,
                            amount * price_in_usdc
                        ),
                    )
                    .parse_mode(teloxide::types::ParseMode::Html)
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
            bot.send_message(
                msg.chat.id,
                "Please enter the recipient's Solana address as text:",
            )
            .await?;
        }
    }

    Ok(())
}

// Handler for amount state
pub async fn receive_withdraw_amount(
    bot: Bot,
    msg: Message,
    state: State,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    if let State::AwaitingWithdrawAmount {
        token_address,
        token_symbol,
        recipient,
        balance,
        price_in_sol,
        price_in_usdc,
    } = state
    {
        if let Some(amount_text) = msg.text() {
            let chat_id = msg.chat.id;
            let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);

            // Create interactor
            let db_pool = services.db_pool();
            let solana_client = services.solana_client();
            let price_service = services.price_service();

            let interactor = Arc::new(WithdrawInteractorImpl::new(
                db_pool,
                solana_client,
                price_service,
            ));

            // Validate amount
            match interactor
                .validate_withdraw_amount(amount_text, balance)
                .await
            {
                Ok(amount) => {
                    // Calculate total values
                    let total_sol = amount * price_in_sol;
                    let total_usdc = amount * price_in_usdc;

                    // Update dialogue state
                    dialogue
                        .update(State::AwaitingWithdrawConfirmation {
                            token_address: token_address.clone(),
                            token_symbol: token_symbol.clone(),
                            recipient: recipient.clone(),
                            amount,
                            price_in_sol,
                            total_sol,
                            total_usdc,
                        })
                        .await?;

                    // Format address for display (shortened)
                    let short_address = if recipient.len() > 12 {
                        format!(
                            "{}...{}",
                            &recipient[..6],
                            &recipient[recipient.len() - 6..]
                        )
                    } else {
                        recipient.clone()
                    };

                    // Prompt for confirmation
                    bot.send_message(
                        chat_id,
                        format!(
                            "<b>Confirm Withdrawal</b>\n\n\
                            • Amount: <b>{:.6} {}</b>\n\
                            • Value: <b>{:.6} SOL</b> (${:.2})\n\
                            • To: <code>{}</code>\n\n\
                            Proceed with this withdrawal? (yes/no)",
                            amount, token_symbol, total_sol, total_usdc, short_address
                        ),
                    )
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await?;
                }
                Err(e) => {
                    bot.send_message(chat_id, format!("Invalid amount: {}", e))
                        .await?;
                }
            }
        } else {
            bot.send_message(msg.chat.id, "Please enter the amount as text:")
                .await?;
        }
    }

    Ok(())
}

// Handler for confirmation state
pub async fn receive_withdraw_confirmation(
    bot: Bot,
    msg: Message,
    state: State,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    if let State::AwaitingWithdrawConfirmation {
        token_address,
        token_symbol,
        recipient,
        amount,
        price_in_sol,
        total_sol,
        total_usdc,
    } = state
    {
        if let Some(text) = msg.text() {
            let confirmation = text.to_lowercase();
            let chat_id = msg.chat.id;
            let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);

            // Reset dialogue state
            dialogue.update(State::Start).await?;

            if confirmation == "yes" || confirmation == "y" {
                // Show processing message
                let processing_msg = bot
                    .send_message(chat_id, "Processing withdrawal... Please wait.")
                    .await?;

                // Create interactor
                let db_pool = services.db_pool();
                let solana_client = services.solana_client();
                let price_service = services.price_service();

                let interactor = Arc::new(WithdrawInteractorImpl::new(
                    db_pool,
                    solana_client,
                    price_service,
                ));

                // Execute withdrawal
                let result = interactor
                    .execute_withdraw(
                        telegram_id,
                        &token_address,
                        &token_symbol,
                        &recipient,
                        amount,
                        price_in_sol,
                    )
                    .await?;

                if result.success {
                    // Success message
                    let text = format!(
                        "✅ <b>Withdrawal Successful</b>\n\n\
                        • Amount: <b>{:.6} {}</b>\n\
                        • Recipient: <code>{}</code>\n\
                        • Tx Signature: <code>{}</code>\n\n\
                        <a href=\"https://explorer.solana.com/tx/{}\">View on Explorer</a>",
                        amount,
                        token_symbol,
                        recipient,
                        result.signature.as_deref().unwrap_or("unknown"),
                        result.signature.as_deref().unwrap_or("unknown")
                    );

                    bot.edit_message_text(chat_id, processing_msg.id, text)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                } else {
                    // Error message
                    let text = format!(
                        "❌ <b>Withdrawal Failed</b>\n\n\
                        • Amount: <b>{:.6} {}</b>\n\
                        • Recipient: <code>{}</code>\n\
                        • Error: <code>{}</code>",
                        amount,
                        token_symbol,
                        recipient,
                        result
                            .error_message
                            .unwrap_or_else(|| "Unknown error".to_string())
                    );

                    bot.edit_message_text(chat_id, processing_msg.id, text)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                }
            } else {
                // Cancelled
                bot.send_message(chat_id, "Withdrawal cancelled.").await?;
            }
        } else {
            bot.send_message(msg.chat.id, "Please confirm with 'yes' or 'no' as text:")
                .await?;
        }
    }

    Ok(())
}
