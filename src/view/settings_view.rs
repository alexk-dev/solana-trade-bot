use anyhow::Result;
use async_trait::async_trait;
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode},
    Bot,
};

#[async_trait]
pub trait SettingsView: Send + Sync {
    async fn display_settings_menu(&self, slippage: f64) -> Result<()>;
    async fn display_slippage_prompt(&self, current_slippage: f64) -> Result<()>;
    async fn display_slippage_updated(&self, new_slippage: f64) -> Result<()>;
    async fn display_invalid_slippage(&self, error_message: String) -> Result<()>;
    async fn display_error(&self, error_message: String) -> Result<()>;
}

pub struct TelegramSettingsView {
    bot: Bot,
    chat_id: ChatId,
}

impl TelegramSettingsView {
    pub fn new(bot: Bot, chat_id: ChatId) -> Self {
        Self { bot, chat_id }
    }
}

#[async_trait]
impl SettingsView for TelegramSettingsView {
    async fn display_settings_menu(&self, slippage: f64) -> Result<()> {
        // Create keyboard with settings options
        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![InlineKeyboardButton::callback(
                format!("Slippage ({}%)", slippage),
                "set_slippage",
            )],
            vec![InlineKeyboardButton::callback("Back to Menu", "menu")],
        ]);

        self.bot
            .send_message(
                self.chat_id,
                "<b>Settings</b>\n\nConfigure your trading preferences:".to_string(),
            )
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await?;

        Ok(())
    }

    async fn display_slippage_prompt(&self, current_slippage: f64) -> Result<()> {
        // Provide preset options for common values
        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![
                InlineKeyboardButton::callback("0.1%", "slippage_0.1"),
                InlineKeyboardButton::callback("0.5%", "slippage_0.5"),
                InlineKeyboardButton::callback("1.0%", "slippage_1.0"),
            ],
            vec![
                InlineKeyboardButton::callback("2.0%", "slippage_2.0"),
                InlineKeyboardButton::callback("3.0%", "slippage_3.0"),
                InlineKeyboardButton::callback("5.0%", "slippage_5.0"),
            ],
            vec![InlineKeyboardButton::callback("Cancel", "settings")],
        ]);

        self.bot
            .send_message(
                self.chat_id,
                format!(
                    "Your current slippage tolerance is set to <b>{:.1}%</b>\n\n\
                    Select a preset value or type a custom percentage between 0.1% and 5.0%:",
                    current_slippage
                ),
            )
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await?;

        Ok(())
    }

    async fn display_slippage_updated(&self, new_slippage: f64) -> Result<()> {
        let keyboard = InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
            "Back to Settings",
            "settings",
        )]]);

        self.bot
            .send_message(
                self.chat_id,
                format!(
                    "✅ Slippage tolerance has been updated to <b>{:.1}%</b>",
                    new_slippage
                ),
            )
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await?;

        Ok(())
    }

    async fn display_invalid_slippage(&self, error_message: String) -> Result<()> {
        self.bot
            .send_message(
                self.chat_id,
                format!(
                    "⚠️ Invalid slippage value: {}\n\nPlease enter a number between 0.1 and 5.0",
                    error_message
                ),
            )
            .await?;

        Ok(())
    }

    async fn display_error(&self, error_message: String) -> Result<()> {
        self.bot
            .send_message(self.chat_id, format!("Error: {}", error_message))
            .await?;

        Ok(())
    }
}
