use anyhow::{anyhow, Result};
use log::info;
use std::sync::Arc;
use teloxide::{prelude::*, types::ParseMode};

use super::{CommandHandler, MyDialogue};
use crate::di::ServiceContainer;
use crate::interactor::balance_interactor::BalanceInteractorImpl;
use crate::interactor::db;
use crate::presenter::balance_presenter::{BalancePresenter, BalancePresenterImpl};
use crate::view::balance_view::TelegramBalanceView;

pub struct MenuCommand;

impl CommandHandler for MenuCommand {
    fn command_name() -> &'static str {
        "menu"
    }

    fn description() -> &'static str {
        "main menu"
    }

    async fn execute(
        bot: Bot,
        msg: Message,
        telegram_id: i64,
        _dialogue: Option<MyDialogue>,
        services: Arc<ServiceContainer>,
    ) -> Result<()> {
        let db_pool = services.db_pool();
        let username = msg.from().and_then(|user| user.username.clone());
        let chat_id = msg.chat.id;

        info!("Menu command received from Telegram ID: {}", telegram_id);

        let user_exists = db::check_user_exists(&db_pool, telegram_id)
            .await
            .map_err(|e| anyhow!("Database error: {}", e))?;

        // Register user if they don't exist
        if !user_exists {
            db::create_user(&db_pool, telegram_id, username)
                .await
                .map_err(|e| anyhow!("Failed to create user: {}", e))?;

            bot.send_message(
                chat_id,
                "<b>Hello!</b> I'm a Solana trading bot. You have been successfully registered.",
            )
            .parse_mode(ParseMode::Html)
            .await?;
        }

        let solana_client = services.solana_client();
        let price_service = services.price_service();
        let interactor = Arc::new(BalanceInteractorImpl::new(
            db_pool.clone(),
            solana_client,
            price_service,
        ));
        let view = Arc::new(TelegramBalanceView::new(bot, chat_id));
        let presenter = BalancePresenterImpl::new(interactor, view);

        presenter.show_balances(telegram_id).await?;

        Ok(())
    }
}
