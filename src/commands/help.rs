use super::{CommandHandler, MyDialogue};
use crate::di::ServiceContainer;
use anyhow::Result;
use std::sync::Arc;
use teloxide::prelude::*;

pub struct HelpCommand;

impl CommandHandler for HelpCommand {
    fn command_name() -> &'static str {
        "help"
    }

    fn description() -> &'static str {
        "display this help message"
    }

    async fn execute(
        bot: Bot,
        msg: Message,
        telegram_id: i64,
        _dialogue: Option<MyDialogue>,
        _services: Arc<ServiceContainer>,
    ) -> Result<()> {
        bot.send_message(
            msg.chat.id,
            "Available commands:\n\
            /start - Start working with the bot\n\
            /menu - Main menu\n\
            /help - Show this help",
        )
        .await?;

        Ok(())
    }
}
