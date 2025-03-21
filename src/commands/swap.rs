use anyhow::Result;
use log;
use std::sync::Arc;
use teloxide::prelude::*;

use super::{CommandHandler, MyDialogue};
use crate::di::ServiceContainer;
use crate::entity::State;
use crate::interactor::swap_interactor::SwapInteractorImpl;
use crate::presenter::swap_presenter::{SwapPresenter, SwapPresenterImpl};
use crate::view::swap_view::TelegramSwapView;

pub struct SwapCommand;

impl CommandHandler for SwapCommand {
    fn command_name() -> &'static str {
        "swap"
    }

    fn description() -> &'static str {
        "swap tokens via Jupiter DEX (format: /swap amount from_token to_token slippage%)"
    }

    async fn execute(
        bot: Bot,
        msg: Message,
        _dialogue: Option<MyDialogue>,
        services: Arc<ServiceContainer>,
    ) -> Result<()> {
        let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);
        let chat_id = msg.chat.id;

        // Get command parts
        let command_parts: Vec<&str> = msg.text().unwrap_or("").split_whitespace().collect();

        let db_pool = services.db_pool();
        let solana_client = services.solana_client();
        let swap_service = services.swap_service();
        let token_repository = services.token_repository();

        let interactor = Arc::new(SwapInteractorImpl::new(
            db_pool,
            solana_client,
            swap_service,
            token_repository,
        ));
        let view = Arc::new(TelegramSwapView::new(bot, chat_id));
        let presenter = SwapPresenterImpl::new(interactor, view);
        presenter
            .process_swap_command(telegram_id, command_parts)
            .await
    }
}

pub async fn receive_swap_details(bot: Bot, msg: Message, dialogue: MyDialogue) -> Result<()> {
    dialogue.update(State::Start).await?;
    bot.send_message(
        msg.chat.id,
        "The token swap feature via chat is under development (placeholder).",
    )
    .await?;
    Ok(())
}
