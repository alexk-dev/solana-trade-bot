use crate::entity::OrderType;
use anyhow::Result;
use async_trait::async_trait;
use teloxide::{prelude::*, Bot};

#[async_trait]
pub trait TradeView: Send + Sync {
    async fn prompt_for_token_address(&self, trade_type: &OrderType) -> Result<()>;
    async fn display_invalid_token_address(&self) -> Result<()>;
    async fn display_token_info(
        &self,
        order_type: &OrderType,
        token_address: &str,
        token_symbol: &str,
        price_in_sol: f64,
        price_in_usdc: f64,
    ) -> Result<()>;
    async fn display_invalid_amount(&self, error_message: String) -> Result<()>;
    async fn prompt_for_confirmation(
        &self,
        order_type: &OrderType,
        token_address: &str,
        token_symbol: &str,
        amount: f64,
        price_in_sol: f64,
        total_sol: f64,
    ) -> Result<()>;
    async fn display_processing(&self, trade_type: &OrderType) -> Result<Option<Message>>;
    async fn display_trade_success(
        &self,
        trade_type: &OrderType,
        token_symbol: &str,
        amount: f64,
        price_in_sol: f64,
        total_sol: f64,
        signature: &str,
        message: Option<Message>,
    ) -> Result<()>;
    async fn display_trade_error(
        &self,
        trade_type: &OrderType,
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
    async fn prompt_for_token_address(&self, trade_type: &OrderType) -> Result<()> {
        self.bot
            .send_message(
                self.chat_id,
                format!(
                    "Please enter the token contract address you want to {}:",
                    trade_type.to_string().to_lowercase()
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
        order_type: &OrderType,
        token_address: &str,
        token_symbol: &str,
        current_price_in_sol: f64,
        current_price_in_usdc: f64,
    ) -> Result<()> {
        let action = match order_type {
            OrderType::Buy => "buy",
            OrderType::Sell => "sell",
        };

        let additional_instructions = if *order_type == OrderType::Sell {
            "\n\nFor sell orders, you can also specify a percentage of your holdings:\n<price> <percentage>%\nExample: 0.5 50% (sell 50% of your tokens at 0.5 SOL each)"
        } else {
            ""
        };

        self.bot
            .send_message(
                self.chat_id,
                format!(
                    "Token: {} ({})\nCurrent price: {:.6} SOL (${:.2})\n\nPlease enter the price in SOL and total volume in SOL to {} in the format:\n<price> <volume_in_sol>\nExample: 0.5 10 (10 SOL volume at price 0.5 SOL per token){}",
                    token_symbol, token_address, current_price_in_sol, current_price_in_usdc, action, additional_instructions
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
        order_type: &OrderType,
        token_address: &str,
        token_symbol: &str,
        price_in_sol: f64,
        amount: f64,
        total_sol: f64,
    ) -> Result<()> {
        let order_type_str = match order_type {
            OrderType::Buy => "BUY",
            OrderType::Sell => "SELL",
        };

        self.bot
            .send_message(
                self.chat_id,
                format!(
                    "Please confirm your limit order:\n\n{} {:.6} SOL ({:.6} {} tokens) @ {:.6} SOL each\n\nDo you want to proceed? (yes/no)",
                    order_type_str, total_sol, amount, token_symbol, price_in_sol
                ),
            )
            .await?;
        Ok(())
    }

    async fn display_processing(&self, trade_type: &OrderType) -> Result<Option<Message>> {
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
        trade_type: &OrderType,
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
        trade_type: &OrderType,
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
