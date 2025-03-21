use super::{CommandHandler, MyDialogue};
use crate::di::ServiceContainer;
use crate::interactor::price_interactor::{PriceInteractor, PriceInteractorImpl};
use crate::presenter::price_presenter::{PricePresenter, PricePresenterImpl};
use crate::view::price_view::{PriceView, TelegramPriceView};
use anyhow::Result;
use log::info;
use solana_client::nonblocking::rpc_client::RpcClient;
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
        _dialogue: Option<MyDialogue>,
        _solana_client: Option<Arc<RpcClient>>,
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

            // Execute the use case via presenter
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
