use crate::entity::WatchlistItem;
use anyhow::Result;
use async_trait::async_trait;
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode},
    Bot,
};

#[async_trait]
pub trait WatchlistView: Send + Sync {
    async fn display_watchlist(&self, watchlist: Vec<WatchlistItem>) -> Result<()>;
    async fn display_token_detail(
        &self,
        item: WatchlistItem,
        price_in_usdc: Option<f64>,
    ) -> Result<()>;
    async fn display_empty_watchlist(&self) -> Result<()>;
    async fn prompt_for_token_address(&self) -> Result<()>;
    async fn display_token_added(&self, item: WatchlistItem) -> Result<()>;
    async fn display_token_removed(&self, token_symbol: &str) -> Result<()>;
    async fn display_invalid_token_address(&self, error_message: String) -> Result<()>;
    async fn display_error(&self, error_message: String) -> Result<()>;
}

pub struct TelegramWatchlistView {
    bot: Bot,
    chat_id: ChatId,
}

impl TelegramWatchlistView {
    pub fn new(bot: Bot, chat_id: ChatId) -> Self {
        Self { bot, chat_id }
    }
}

#[async_trait]
impl WatchlistView for TelegramWatchlistView {
    async fn display_watchlist(&self, watchlist: Vec<WatchlistItem>) -> Result<()> {
        if watchlist.is_empty() {
            return self.display_empty_watchlist().await;
        }

        // Create buttons for each token in the watchlist
        let mut keyboard_buttons = vec![];

        for item in watchlist {
            let button_text = format!(
                "{}: {} SOL",
                item.token_symbol,
                format!("{:.6}", item.last_price_in_sol)
            );

            keyboard_buttons.push(vec![InlineKeyboardButton::callback(
                button_text,
                format!("watchlist_view_{}", item.token_address),
            )]);
        }

        // Add Add and Back buttons
        keyboard_buttons.push(vec![
            InlineKeyboardButton::callback("➕ Add to List", "watchlist_add"),
            InlineKeyboardButton::callback("🔄 Refresh", "watchlist_refresh"),
        ]);
        keyboard_buttons.push(vec![InlineKeyboardButton::callback(
            "← Back to Menu",
            "menu",
        )]);

        let keyboard = InlineKeyboardMarkup::new(keyboard_buttons);

        self.bot
            .send_message(
                self.chat_id,
                "<b>Your Watchlist</b>\n\nSelect a token for details or add new ones:",
            )
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await?;

        Ok(())
    }

    async fn display_token_detail(
        &self,
        item: WatchlistItem,
        price_in_usdc: Option<f64>,
    ) -> Result<()> {
        let usdc_price_text = if let Some(price) = price_in_usdc {
            format!("${:.6} USD", price)
        } else {
            "USD price unavailable".to_string()
        };

        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![InlineKeyboardButton::callback(
                "🗑️ Remove from Watchlist",
                format!("watchlist_remove_{}", item.token_address),
            )],
            vec![InlineKeyboardButton::callback(
                "← Back to Watchlist",
                "watchlist",
            )],
        ]);

        self.bot
            .send_message(
                self.chat_id,
                format!(
                    "<b>{} Token Details</b>\n\n\
                    • Symbol: <b>{}</b>\n\
                    • Address: <code>{}</code>\n\
                    • Current Price: <b>{:.6} SOL</b> ({})\n\
                    • Added: {}\n\
                    • Last Updated: {}",
                    item.token_symbol,
                    item.token_symbol,
                    item.token_address,
                    item.last_price_in_sol,
                    usdc_price_text,
                    item.created_at.format("%Y-%m-%d %H:%M"),
                    item.updated_at.format("%Y-%m-%d %H:%M")
                ),
            )
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await?;

        Ok(())
    }

    async fn display_empty_watchlist(&self) -> Result<()> {
        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![InlineKeyboardButton::callback(
                "➕ Add First Token",
                "watchlist_add",
            )],
            vec![InlineKeyboardButton::callback("← Back to Menu", "menu")],
        ]);

        self.bot
            .send_message(
                self.chat_id,
                "Your watchlist is empty. Add tokens to track their prices!",
            )
            .reply_markup(keyboard)
            .await?;

        Ok(())
    }

    async fn prompt_for_token_address(&self) -> Result<()> {
        self.bot
            .send_message(
                self.chat_id,
                "Please enter the token contract address you want to add to your watchlist:",
            )
            .await?;

        Ok(())
    }

    async fn display_token_added(&self, item: WatchlistItem) -> Result<()> {
        let keyboard = InlineKeyboardMarkup::new(vec![vec![
            InlineKeyboardButton::callback("View Watchlist", "watchlist"),
            InlineKeyboardButton::callback("Add Another", "watchlist_add"),
        ]]);

        self.bot
            .send_message(
                self.chat_id,
                format!(
                    "✅ Added <b>{}</b> to your watchlist\nCurrent price: <b>{:.6} SOL</b>",
                    item.token_symbol, item.last_price_in_sol
                ),
            )
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await?;

        Ok(())
    }

    async fn display_token_removed(&self, token_symbol: &str) -> Result<()> {
        let keyboard = InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
            "Back to Watchlist",
            "watchlist",
        )]]);

        self.bot
            .send_message(
                self.chat_id,
                format!("✅ Removed {} from your watchlist", token_symbol),
            )
            .reply_markup(keyboard)
            .await?;

        Ok(())
    }

    async fn display_invalid_token_address(&self, error_message: String) -> Result<()> {
        let keyboard = InlineKeyboardMarkup::new(vec![vec![
            InlineKeyboardButton::callback("Try Again", "watchlist_add"),
            InlineKeyboardButton::callback("Cancel", "watchlist"),
        ]]);

        self.bot
            .send_message(
                self.chat_id,
                format!(
                    "❌ Invalid token address: {}\n\nPlease enter a valid Solana token address.",
                    error_message
                ),
            )
            .reply_markup(keyboard)
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
