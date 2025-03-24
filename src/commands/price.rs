use super::{CommandHandler, MyDialogue};
use crate::di::ServiceContainer;
use crate::interactor::price_interactor::PriceInteractorImpl;
use crate::presenter::price_presenter::{PricePresenter, PricePresenterImpl};
use crate::view::price_view::TelegramPriceView;
use anyhow::Result;
use log::info;
use std::sync::Arc;
use teloxide::prelude::*;

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
