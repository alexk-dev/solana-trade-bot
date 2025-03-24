use anyhow::Result;
use async_trait::async_trait;
use teloxide::{prelude::*, Bot};

#[async_trait]
pub trait TradeView: Send + Sync {
    async fn prompt_for_token_address(&self, trade_type: &str) -> Result<()>;
    async fn display_invalid_token_address(&self) -> Result<()>;
    async fn display_token_info(
        &self,
        trade_type: &str,
        token_address: &str,
        token_symbol: &str,
        price_in_sol: f64,
        price_in_usdc: f64,
    ) -> Result<()>;
    async fn display_invalid_amount(&self, error_message: String) -> Result<()>;
    async fn prompt_for_confirmation(
        &self,
        trade_type: &str,
        token_address: &str,
        token_symbol: &str,
        amount: f64,
        price_in_sol: f64,
        total_sol: f64,
    ) -> Result<()>;
    async fn display_processing(&self, trade_type: &str) -> Result<Option<Message>>;
    async fn display_trade_success(
        &self,
        trade_type: &str,
        token_symbol: &str,
        amount: f64,
        price_in_sol: f64,
        total_sol: f64,
        signature: &str,
        message: Option<Message>,
    ) -> Result<()>;
    async fn display_trade_error(
        &self,
        trade_type: &str,
        token_symbol: &str,
        amount: f64,
        error_message: String,
        message: Option<Message>,
    ) -> Result<()>;
    async fn display_trade_cancelled(&self) -> Result<()>;
    async fn display_error(&self, error_message: String) -> Result<()>;
}

pub struct TelegramTradeView {
    bot: Bot,
    chat_id: ChatId,
}

impl TelegramTradeView {
    pub fn new(bot: Bot, chat_id: ChatId) -> Self {
        Self { bot, chat_id }
    }
}

#[async_trait]
impl TradeView for TelegramTradeView {
    async fn prompt_for_token_address(&self, trade_type: &str) -> Result<()> {
        self.bot
            .send_message(
                self.chat_id,
                format!(
                    "Please enter the token contract address you want to {}:",
                    trade_type.to_lowercase()
                ),
            )
            .await?;
        Ok(())
    }

    async fn display_invalid_token_address(&self) -> Result<()> {
        self.bot
            .send_message(
                self.chat_id,
                "Invalid token address. Please enter a valid Solana token contract address:",
            )
            .await?;
        Ok(())
    }

    async fn display_token_info(
        &self,
        trade_type: &str,
        token_address: &str,
        token_symbol: &str,
        price_in_sol: f64,
        price_in_usdc: f64,
    ) -> Result<()> {
        self.bot
            .send_message(
                self.chat_id,
                format!(
                    "Token: {} ({})\nCurrent price: {:.6} SOL (${:.2})\n\nHow many tokens do you want to {}?",
                    token_symbol, token_address, price_in_sol, price_in_usdc, trade_type.to_lowercase()
                ),
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
        trade_type: &str,
        token_address: &str,
        token_symbol: &str,
        amount: f64,
        price_in_sol: f64,
        total_sol: f64,
    ) -> Result<()> {
        self.bot
            .send_message(
                self.chat_id,
                format!(
                    "Please confirm your trade:\n\n{} {} {}\nPrice per token: {:.6} SOL\nTotal: {:.6} SOL\n\nDo you want to proceed? (yes/no)",
                    trade_type, amount, token_symbol, price_in_sol, total_sol
                ),
            )
            .await?;
        Ok(())
    }

    async fn display_processing(&self, trade_type: &str) -> Result<Option<Message>> {
        let message = self
            .bot
            .send_message(
                self.chat_id,
                format!("Processing your {} order... Please wait.", trade_type),
            )
            .await?;

        Ok(Some(message))
    }

    async fn display_trade_success(
        &self,
        trade_type: &str,
        token_symbol: &str,
        amount: f64,
        price_in_sol: f64,
        total_sol: f64,
        signature: &str,
        message: Option<Message>,
    ) -> Result<()> {
        let text = format!(
            "✅ {} order completed successfully.\nAmount: {} {}\nPrice: {:.6} SOL per token\nTotal: {:.6} SOL\nTx Signature: {}\nCheck transaction: https://explorer.solana.com/tx/{}",
            trade_type, amount, token_symbol, price_in_sol, total_sol, signature, signature
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

    async fn display_trade_error(
        &self,
        trade_type: &str,
        token_symbol: &str,
        amount: f64,
        error_message: String,
        message: Option<Message>,
    ) -> Result<()> {
        let text = format!(
            "❌ Error executing {} order for {} {}:\n{}",
            trade_type, amount, token_symbol, error_message
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

    async fn display_trade_cancelled(&self) -> Result<()> {
        self.bot
            .send_message(self.chat_id, "Trade cancelled.")
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
