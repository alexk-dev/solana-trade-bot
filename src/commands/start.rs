// src/commands/start.rs
use anyhow::Result;
use log::info;
use sqlx::PgPool;
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use teloxide::{prelude::*, types::ParseMode};

use crate::db;
use crate::MyDialogue;
use super::CommandHandler;

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
        db_pool: Option<PgPool>,
        _solana_client: Option<Arc<RpcClient>>
    ) -> Result<()> {
        let db_pool = db_pool.ok_or_else(|| anyhow::anyhow!("Database pool not provided"))?;
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
                "Привет! Я бот для управления Solana-кошельком. Вы успешно зарегистрированы.\n\n\
                Используйте /create_wallet чтобы создать новый кошелек, или /help для просмотра всех команд."
            )
                .parse_mode(ParseMode::Markdown)
                .await?;
        } else {
            bot.send_message(
                msg.chat.id,
                "С возвращением! Используйте /help для просмотра доступных команд."
            )
                .parse_mode(ParseMode::Markdown)
                .await?;
        }

        Ok(())
    }
}