use anyhow::Result;
use async_trait::async_trait;
use teloxide::{
    prelude::*,
    types::{MessageId, ParseMode},
    Bot,
};

#[async_trait]
pub trait SendView: Send + Sync {
    async fn prompt_for_recipient_address(&self) -> Result<()>;
    async fn display_invalid_address(&self) -> Result<()>;
    async fn prompt_for_amount(&self) -> Result<()>;
    async fn display_invalid_amount(&self, error_message: String) -> Result<()>;
    async fn prompt_for_confirmation(
        &self,
        recipient: &str,
        amount: f64,
        token: &str,
    ) -> Result<()>;
    async fn display_processing(&self) -> Result<Option<Message>>;
    async fn display_transaction_success(
        &self,
        recipient: &str,
        amount: f64,
        token: &str,
        signature: &str,
        message: Option<Message>,
    ) -> Result<()>;
    async fn display_transaction_error(
        &self,
        recipient: &str,
        amount: f64,
        token: &str,
        error_message: String,
        message: Option<Message>,
    ) -> Result<()>;
    async fn display_transaction_cancelled(&self) -> Result<()>;
    async fn display_no_wallet(&self) -> Result<()>;
}

pub struct TelegramSendView {
    bot: Bot,
    chat_id: ChatId,
}

impl TelegramSendView {
    pub fn new(bot: Bot, chat_id: ChatId) -> Self {
        Self { bot, chat_id }
    }
}

#[async_trait]
impl SendView for TelegramSendView {
    async fn prompt_for_recipient_address(&self) -> Result<()> {
        self.bot
            .send_message(self.chat_id, "Enter the recipient's Solana address:")
            .await?;
        Ok(())
    }

    async fn display_invalid_address(&self) -> Result<()> {
        self.bot
            .send_message(
                self.chat_id,
                "Invalid Solana address. Please check the address and try again:",
            )
            .await?;
        Ok(())
    }

    async fn prompt_for_amount(&self) -> Result<()> {
        self.bot
            .send_message(
                self.chat_id,
                "Enter the amount to send (example: 0.5 SOL or 100 USDC):",
            )
            .await?;
        Ok(())
    }

    async fn display_invalid_amount(&self, error_message: String) -> Result<()> {
        self.bot
            .send_message(self.chat_id, format!("Invalid amount: {}", error_message))
            .await?;
        Ok(())
    }

    async fn prompt_for_confirmation(
        &self,
        recipient: &str,
        amount: f64,
        token: &str,
    ) -> Result<()> {
        self.bot
            .send_message(
                self.chat_id,
                format!(
                    "Confirm sending {} {} to address {} (yes/no):",
                    amount, token, recipient
                ),
            )
            .await?;
        Ok(())
    }

    async fn display_processing(&self) -> Result<Option<Message>> {
        let message = self
            .bot
            .send_message(self.chat_id, "Sending funds... Please wait.")
            .await?;

        Ok(Some(message))
    }

    async fn display_transaction_success(
        &self,
        recipient: &str,
        amount: f64,
        token: &str,
        signature: &str,
        message: Option<Message>,
    ) -> Result<()> {
        let text = format!(
            "✅ Funds sent successfully.\nAmount: {} {}\nTo: {}\nTx Signature: {}\nCheck transaction: https://explorer.solana.com/tx/{}",
            amount, token, recipient, signature, signature
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

    async fn display_transaction_error(
        &self,
        recipient: &str,
        amount: f64,
        token: &str,
        error_message: String,
        message: Option<Message>,
    ) -> Result<()> {
        let text = format!(
            "❌ Error sending {} {} to {}:\n{}",
            amount, token, recipient, error_message
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

    async fn display_transaction_cancelled(&self) -> Result<()> {
        let text = "Transaction cancelled.";
        self.bot.send_message(self.chat_id, text).await?;

        Ok(())
    }

    async fn display_no_wallet(&self) -> Result<()> {
        self.bot
            .send_message(
                self.chat_id,
                "You don't have a wallet yet. Use /create_wallet to create a new wallet.",
            )
            .await?;
        Ok(())
    }
}
