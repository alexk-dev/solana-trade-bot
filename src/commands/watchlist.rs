use anyhow::Result;
use log::info;
use std::sync::Arc;
use teloxide::prelude::*;

use super::{CommandHandler, MyDialogue};
use crate::di::ServiceContainer;
use crate::entity::State;
use crate::interactor::watchlist_interactor::WatchlistInteractorImpl;
use crate::presenter::watchlist_presenter::{WatchlistPresenter, WatchlistPresenterImpl};
use crate::view::watchlist_view::TelegramWatchlistView;

pub struct WatchlistCommand;

impl CommandHandler for WatchlistCommand {
    fn command_name() -> &'static str {
        "watchlist"
    }

    fn description() -> &'static str {
        "manage your token watchlist"
    }

    async fn execute(
        bot: Bot,
        msg: Message,
        telegram_id: i64,
        dialogue: Option<MyDialogue>,
        services: Arc<ServiceContainer>,
    ) -> Result<()> {
        let chat_id = msg.chat.id;

        info!(
            "Watchlist command received from Telegram ID: {}",
            telegram_id
        );

        let db_pool = services.db_pool();
        let price_service = services.price_service();
        let token_repository = services.token_repository();

        let interactor = Arc::new(WatchlistInteractorImpl::new(
            db_pool,
            price_service.clone(),
            token_repository,
        ));
        let view = Arc::new(TelegramWatchlistView::new(bot, chat_id));
        let presenter = WatchlistPresenterImpl::new(interactor, view, price_service);

        presenter.show_watchlist(telegram_id).await?;

        Ok(())
    }
}

// Handler for token address input when adding to watchlist
pub async fn handle_watchlist_token_address(
    bot: Bot,
    msg: Message,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let chat_id = msg.chat.id;
    let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);

    // Reset dialogue state
    dialogue.update(State::Start).await?;

    if let Some(token_address) = msg.text() {
        let db_pool = services.db_pool();
        let price_service = services.price_service();
        let token_repository = services.token_repository();

        let interactor = Arc::new(WatchlistInteractorImpl::new(
            db_pool,
            price_service.clone(),
            token_repository,
        ));
        let view = Arc::new(TelegramWatchlistView::new(bot.clone(), chat_id));
        let presenter = WatchlistPresenterImpl::new(interactor, view, price_service);

        presenter
            .add_to_watchlist(telegram_id, token_address)
            .await?;
    } else {
        bot.send_message(chat_id, "Please enter a valid token address.")
            .await?;
    }

    Ok(())
}
