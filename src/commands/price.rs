use super::{CommandHandler, MyDialogue};
use crate::di::ServiceContainer;
use crate::interactor::price_interactor::PriceInteractorImpl;
use crate::presenter::price_presenter::{PricePresenter, PricePresenterImpl};
use crate::view::price_view::TelegramPriceView;
use crate::State;
use anyhow::Result;
use log::info;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub struct PriceCommand;

impl CommandHandler for PriceCommand {
    fn command_name() -> &'static str {
        "price"
    }

    fn description() -> &'static str {
        "get price for a token"
    }

    async fn execute(
        bot: Bot,
        msg: Message,
        telegram_id: i64,
        _dialogue: Option<MyDialogue>,
        services: Arc<ServiceContainer>,
    ) -> Result<()> {
        let command_parts: Vec<&str> = msg.text().unwrap_or("").split_whitespace().collect();
        let chat_id = msg.chat.id;

        if command_parts.len() >= 2 {
            let token = command_parts[1];

            info!("Price command received for token: {}", token);

            let price_service = services.price_service();
            let interactor = Arc::new(PriceInteractorImpl::new(price_service));
            let view = Arc::new(TelegramPriceView::new(bot, chat_id));
            let presenter = PricePresenterImpl::new(interactor, view);

            presenter.show_token_price(token).await
        } else {
            bot.send_message(
                chat_id,
                "Use the command in this format: /price <token_symbol>\n\nExample: /price SOL",
            )
            .await?;

            Ok(())
        }
    }
}

pub async fn receive_price_token_address(
    bot: Bot,
    msg: Message,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    if let Some(address_text) = msg.text() {
        let chat_id = msg.chat.id;

        // Reset dialogue state
        dialogue.update(State::Start).await?;

        // Get price service
        let price_service = services.price_service();
        let token_repository = services.token_repository();

        // Validate token address using the token repository
        match token_repository.get_token_by_id(address_text).await {
            Ok(token) => {
                // Token exists, get price information
                let loading_msg = bot
                    .send_message(chat_id, format!("Getting price for {}...", token.symbol))
                    .await?;

                match price_service.get_token_price(address_text).await {
                    Ok(price_info) => {
                        // Format price message
                        let price_text = format!(
                            "Current price for {}:\n• {:.6} SOL\n• ${:.6} USDC",
                            token.symbol, price_info.price_in_sol, price_info.price_in_usdc
                        );

                        // Create a button to return to main menu
                        let keyboard = InlineKeyboardMarkup::new(vec![vec![
                            InlineKeyboardButton::callback("Check Another Price", "price"),
                            InlineKeyboardButton::callback("← Back to Menu", "menu"),
                        ]]);

                        // Update loading message with price info
                        bot.edit_message_text(chat_id, loading_msg.id, price_text)
                            .reply_markup(keyboard)
                            .await?;
                    }
                    Err(e) => {
                        bot.edit_message_text(
                            chat_id,
                            loading_msg.id,
                            format!("Error getting price: {}", e),
                        )
                        .await?;
                    }
                }
            }
            Err(_) => {
                // Invalid token
                bot.send_message(
                    chat_id,
                    "Invalid token address. Please enter a valid Solana token contract address or use the menu.",
                )
                    .await?;
            }
        }
    } else {
        bot.send_message(msg.chat.id, "Please enter a token address as text.")
            .await?;
    }

    Ok(())
}
