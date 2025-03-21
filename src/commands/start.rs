// src/commands/start.rs
use anyhow::Result;
use log::info;
use solana_client::nonblocking::rpc_client::RpcClient;
use sqlx::PgPool;
use std::sync::Arc;
use teloxide::{prelude::*, types::ParseMode};

use super::CommandHandler;
use crate::db;
use crate::di::ServiceContainer;
use crate::MyDialogue;

pub struct StartCommand;

impl CommandHandler for StartCommand {
    fn command_name() -> &'static str {
        "start"
    }

    fn description() -> &'static str {
        "start the bot"
    }

    async fn execute(
        bot: Bot,
        msg: Message,
        _dialogue: Option<MyDialogue>,
        _solana_client: Option<Arc<RpcClient>>,
        services: Arc<ServiceContainer>,
    ) -> Result<()> {
        let db_pool = services.db_pool();

        let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);
        let username = msg.from().and_then(|user| user.username.clone());

        info!("Start command received from Telegram ID: {}", telegram_id);

        let user_exists = db::check_user_exists(&db_pool, telegram_id)
            .await
            .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;

        if !user_exists {
            db::create_user(&db_pool, telegram_id, username)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create user: {}", e))?;

            bot.send_message(
                msg.chat.id,
                "Hello! I'm a Solana wallet management bot. You have been successfully registered.\n\n\
                Use /create_wallet to create a new wallet, or /help to view all available commands."
            )
                .parse_mode(ParseMode::Markdown)
                .await?;
        } else {
            bot.send_message(
                msg.chat.id,
                "Welcome back! Use /help to view all available commands.",
            )
            .parse_mode(ParseMode::Markdown)
            .await?;
        }

        Ok(())
    }
}
