use anyhow::Result;
use async_trait::async_trait;
use std::format;
use teloxide::{prelude::*, types::MessageId, Bot};

#[async_trait]
pub trait SwapView: Send + Sync {
    async fn display_usage(&self) -> Result<()>;
    async fn display_processing(
        &self,
        source_token: &str,
        target_token: &str,
        amount: f64,
    ) -> Result<Option<Message>>;
    async fn display_swap_success(
        &self,
        source_token: &str,
        target_token: &str,
        amount_in: f64,
        amount_out: f64,
        signature: &str,
        message: Option<Message>,
    ) -> Result<()>;
    async fn display_swap_error(
        &self,
        source_token: &str,
        target_token: &str,
        amount_in: f64,
        error_message: String,
        message: Option<Message>,
    ) -> Result<()>;
    async fn display_validation_error(&self, error_message: String) -> Result<()>;
}

pub struct TelegramSwapView {
    bot: Bot,
    chat_id: ChatId,
}

impl TelegramSwapView {
    pub fn new(bot: Bot, chat_id: ChatId) -> Self {
        Self { bot, chat_id }
    }
}

#[async_trait]
impl SwapView for TelegramSwapView {
    async fn display_usage(&self) -> Result<()> {
        self.bot.send_message(
            self.chat_id,
            "Use the command in this format: /swap <amount> <source_token> <target_token> [<slippage>%]\n\n\
             Example: /swap 1.5 SOL USDC 0.5%"
        ).await?;

        Ok(())
    }

    async fn display_processing(
        &self,
        source_token: &str,
        target_token: &str,
        amount: f64,
    ) -> Result<Option<Message>> {
        let message = self
            .bot
            .send_message(
                self.chat_id,
                format!(
                    "Preparing swap of {} {} to {}... Getting quote...",
                    amount, source_token, target_token
                ),
            )
            .await?;

        Ok((Some(message)))
    }

    async fn display_swap_success(
        &self,
        source_token: &str,
        target_token: &str,
        amount_in: f64,
        amount_out: f64,
        signature: &str,
        message: Option<Message>,
    ) -> Result<()> {
        let text = format!(
            "✅ Swap completed successfully!\n\
            Sent: {} {}\n\
            Received: ~{:.6} {}\n\
            Transaction signature: {}\n\
            Check transaction: https://explorer.solana.com/tx/{}",
            amount_in, source_token, amount_out, target_token, signature, signature
        );

        if let Some(msg) = message {
            self.bot
                .edit_message_text(self.chat_id, msg.id, text)
                .await?;
        } else {
            self.bot.send_message(self.chat_id, text).await?;
        }

        Ok(())
    }

    async fn display_swap_error(
        &self,
        source_token: &str,
        target_token: &str,
        amount_in: f64,
        error_message: String,
        message: Option<Message>,
    ) -> Result<()> {
        let text = format!(
            "❌ Error performing swap of {} {} to {}:\n{}\n\n\
            Possible reasons:\n\
            - Insufficient funds for transaction fees\n\
            - Network issues with Solana\n\
            - Transaction rejected by the network",
            amount_in, source_token, target_token, error_message
        );

        if let Some(msg) = message {
            self.bot
                .edit_message_text(self.chat_id, msg.id, text)
                .await?;
        } else {
            self.bot.send_message(self.chat_id, text).await?;
        }

        Ok(())
    }

    async fn display_validation_error(&self, error_message: String) -> Result<()> {
        self.bot
            .send_message(
                self.chat_id,
                format!("❌ Invalid swap parameters: {}", error_message),
            )
            .await?;

        Ok(())
    }
}
