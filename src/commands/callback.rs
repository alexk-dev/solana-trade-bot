use anyhow::Result;
use log::info;
use std::{str::FromStr, sync::Arc};
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode},
};

use crate::commands::{help, price, send, trade, ui, wallet, CommandHandler, MyDialogue};
use crate::di::ServiceContainer;
use crate::entity::State;
use crate::interactor::balance_interactor::{BalanceInteractor, BalanceInteractorImpl};
use crate::interactor::swap_interactor::SwapInteractorImpl;
use crate::interactor::wallet_interactor::WalletInteractorImpl;
use crate::presenter::balance_presenter::{BalancePresenter, BalancePresenterImpl};
use crate::view::balance_view::TelegramBalanceView;

// Main callback handler function
pub async fn handle_callback(
    bot: Bot,
    q: CallbackQuery,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    // Extract the callback data
    let callback_data = match q.clone().data {
        Some(data) => data,
        None => return Ok(()),
    };

    let message = q.regular_message().unwrap();

    // Get the chat ID
    let chat_id = match q.message {
        Some(ref msg) => msg.chat().id,
        None => return Ok(()),
    };

    // Get user's Telegram ID
    let telegram_id = q.from.id.0 as i64;

    info!(
        "Received callback: {} from user {}",
        callback_data, telegram_id
    );

    // Acknowledge the callback query to stop loading animation
    if let Err(err) = bot.answer_callback_query(q.id.clone()).await {
        info!("Failed to answer callback query: {}", err);
    }

    // Process the callback based on its type
    if callback_data == ("menu") || callback_data == "refresh" {
        // Handle refresh action - update balance display
        handle_refresh(&bot, Some(message.clone()), telegram_id, services).await?;
    } else if callback_data == "create_wallet" {
        // Handle create wallet action
        if let msg = message.clone() {
            wallet::CreateWalletCommand::execute(bot, msg, telegram_id, Some(dialogue), services)
                .await?;
        }
    } else if callback_data == "address" {
        // Handle address action
        if let msg = message.clone() {
            wallet::AddressCommand::execute(bot, msg, telegram_id, Some(dialogue), services)
                .await?;
        }
    } else if callback_data == "send" {
        // Handle send action
        if let msg = message.clone() {
            send::SendCommand::execute(bot, msg, telegram_id, Some(dialogue), services).await?;
        }
    } else if callback_data == "price" {
        // Handle price action - show token selection
        handle_check_price(&bot, chat_id, dialogue).await?;
    } else if callback_data.starts_with("price_") {
        // Handle specific token price request
        handle_price_selection(&bot, &callback_data, chat_id, services).await?;
    } else if callback_data == "help" {
        // Handle help action
        if let msg = message.clone() {
            help::HelpCommand::execute(bot, msg, telegram_id, Some(dialogue), services).await?;
        }
    } else if callback_data == "buy" {
        // Handle direct buy command
        trade::BuyCommand::execute(bot, message.clone(), telegram_id, Some(dialogue), services)
            .await?;
    } else if callback_data == "sell" {
        // Handle direct sell command
        trade::SellCommand::execute(bot, message.clone(), telegram_id, Some(dialogue), services)
            .await?;
    } else {
        // Handle trading UI buttons
        bot.send_message(
            chat_id,
            format!("The {} feature is under development.", callback_data),
        )
        .await?;
    }

    Ok(())
}

// Function to show token price selection
async fn handle_check_price(bot: &Bot, chat_id: ChatId, dialogue: MyDialogue) -> Result<()> {
    dialogue.update(State::AwaitingPriceTokenAddress).await?;

    // Prompt user for token address
    bot.send_message(
        chat_id,
        "Please enter the token contract address you want to check the price for:",
    )
    .await?;

    Ok(())
}

// Function to handle token price selection
async fn handle_price_selection(
    bot: &Bot,
    callback_data: &str,
    chat_id: ChatId,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let token = callback_data.strip_prefix("price_").unwrap_or("SOL");

    // Send loading message
    let message = bot
        .send_message(chat_id, format!("Getting price for {}...", token))
        .await?;

    // Call price service
    let price_service = services.price_service();

    match price_service.get_token_price(token).await {
        Ok(price_info) => {
            // Format price message
            let price_text = format!(
                "Current price for {}:\n≈ {:.6} SOL\n≈ ${:.6}",
                price_info.symbol, price_info.price_in_sol, price_info.price_in_usdc
            );

            // Add back button
            let keyboard = InlineKeyboardMarkup::new(vec![vec![
                InlineKeyboardButton::callback("Check Another Price", "price"),
                InlineKeyboardButton::callback("← Back to Menu", "main_menu"),
            ]]);

            // Update message with price info
            bot.edit_message_text(chat_id, message.id, price_text)
                .reply_markup(keyboard)
                .await?;
        }
        Err(e) => {
            // Show error message
            bot.edit_message_text(
                chat_id,
                message.id,
                format!("Error getting price for {}: {}", token, e),
            )
            .await?;
        }
    }

    Ok(())
}

// Function to handle refresh action
async fn handle_refresh(
    bot: &Bot,
    message: Option<Message>,
    telegram_id: i64,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    if let Some(msg) = message {
        let chat_id = msg.chat.id;

        // Create presenter to display refreshed information
        let solana_client = services.solana_client();
        let price_service = services.price_service();
        let interactor = Arc::new(BalanceInteractorImpl::new(
            services.db_pool(),
            solana_client,
            price_service,
        ));
        let view = Arc::new(TelegramBalanceView::new(bot.clone(), chat_id));
        let presenter = BalancePresenterImpl::new(interactor, view);

        // Call the refresh method that updates the existing message
        presenter.refresh_balances(telegram_id, Some(msg)).await?;
    }

    Ok(())
}

// Function to handle swap with predefined amount
async fn handle_swap_amount(
    bot: &Bot,
    callback_data: &str,
    chat_id: ChatId,
    telegram_id: i64,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    // Parse the callback data (format: swap_amount_AMOUNT_SOURCE_to_TARGET)
    let parts: Vec<&str> = callback_data.split('_').collect();

    if parts.len() >= 5 {
        let amount_str = parts[2];
        let source_token = parts[3];
        let target_token = parts[4];

        let amount = match f64::from_str(amount_str) {
            Ok(val) => val,
            Err(_) => {
                bot.send_message(chat_id, "Invalid amount format. Please try again.")
                    .await?;
                return Ok(());
            }
        };

        // Create confirmation keyboard
        let confirm_keyboard = InlineKeyboardMarkup::new(vec![vec![
            InlineKeyboardButton::callback(
                "✅ Confirm Swap",
                format!("confirm_swap_{}_{}_{}", amount, source_token, target_token),
            ),
            InlineKeyboardButton::callback("❌ Cancel", "swap"),
        ]]);

        // Show confirmation message
        bot.send_message(
            chat_id,
            format!(
                "You are about to swap {} {} to {}.\n\nDo you want to proceed?",
                amount, source_token, target_token
            ),
        )
        .reply_markup(confirm_keyboard)
        .await?;
    } else {
        bot.send_message(chat_id, "Invalid swap parameters. Please try again.")
            .await?;
    }

    Ok(())
}
