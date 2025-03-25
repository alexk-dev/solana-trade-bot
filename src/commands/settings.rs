use anyhow::{anyhow, Result};
use log::info;
use std::sync::Arc;
use teloxide::prelude::*;

use super::{CommandHandler, MyDialogue};
use crate::di::ServiceContainer;
use crate::entity::State;
use crate::interactor::settings_interactor::SettingsInteractorImpl;
use crate::presenter::settings_presenter::{SettingsPresenter, SettingsPresenterImpl};
use crate::view::settings_view::TelegramSettingsView;

pub struct SettingsCommand;

impl CommandHandler for SettingsCommand {
    fn command_name() -> &'static str {
        "settings"
    }

    fn description() -> &'static str {
        "configure trading settings"
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
            "Settings command received from Telegram ID: {}",
            telegram_id
        );

        let db_pool = services.db_pool();
        let interactor = Arc::new(SettingsInteractorImpl::new(db_pool));
        let view = Arc::new(TelegramSettingsView::new(bot, chat_id));
        let presenter = SettingsPresenterImpl::new(interactor, view);

        presenter.show_settings_menu(telegram_id).await?;

        Ok(())
    }
}

// State for slippage setting
pub async fn handle_slippage_input(
    bot: Bot,
    msg: Message,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let chat_id = msg.chat.id;
    let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);

    // Reset dialogue state
    dialogue.update(State::Start).await?;

    // Process slippage input
    if let Some(slippage_text) = msg.text() {
        let db_pool = services.db_pool();
        let interactor = Arc::new(SettingsInteractorImpl::new(db_pool));
        let view = Arc::new(TelegramSettingsView::new(bot.clone(), chat_id));
        let presenter = SettingsPresenterImpl::new(interactor, view);

        presenter
            .update_slippage(telegram_id, slippage_text)
            .await?;
    } else {
        bot.send_message(chat_id, "Please enter a valid slippage percentage.")
            .await?;
    }

    Ok(())
}
