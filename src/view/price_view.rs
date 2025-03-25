use anyhow::Result;
use async_trait::async_trait;
use std::format;
use teloxide::{prelude::*, types::MessageId, Bot};

#[async_trait]
pub trait PriceView: Send + Sync {
    async fn display_loading(&self, token_id: &str) -> Result<Option<Message>>;
    async fn display_price(
        &self,
        token_id: &str,
        symbol: &str,
        price_in_sol: f64,
        price_in_usdc: f64,
    ) -> Result<()>;
    async fn display_error(&self, error_message: String) -> Result<()>;
}

pub struct TelegramPriceView {
    bot: Bot,
    chat_id: ChatId,
    loading_message_id: Option<MessageId>,
}

impl TelegramPriceView {
    pub fn new(bot: Bot, chat_id: ChatId) -> Self {
        Self {
            bot,
            chat_id,
            loading_message_id: None,
        }
    }
}

#[async_trait]
impl PriceView for TelegramPriceView {
    async fn display_loading(&self, token_id: &str) -> Result<Option<Message>> {
        let message = self
            .bot
            .send_message(self.chat_id, format!("Getting price for {}...", token_id))
            .await?;

        Ok(Some(message))
    }

    async fn display_price(
        &self,
        token_id: &str,
        symbol: &str,
        price_in_sol: f64,
        price_in_usdc: f64,
    ) -> Result<()> {
        let token_text = if symbol.is_empty() || symbol == "Unknown" {
            token_id.to_string()
        } else {
            symbol.to_string()
        };
        let text = format!(
            "Current price for {}:\n≈ {:.6} SOL\n≈ {:.6} USDC",
            token_text, price_in_sol, price_in_usdc
        );

        if let Some(message_id) = self.loading_message_id {
            self.bot
                .edit_message_text(self.chat_id, message_id, text)
                .await?;
        } else {
            self.bot.send_message(self.chat_id, text).await?;
        }

        Ok(())
    }

    async fn display_error(&self, error_message: String) -> Result<()> {
        let text = format!("❌ Error getting price: {}", error_message);

        if let Some(message_id) = self.loading_message_id {
            self.bot
                .edit_message_text(self.chat_id, message_id, text)
                .await?;
        } else {
            self.bot.send_message(self.chat_id, text).await?;
        }

        Ok(())
    }
}
