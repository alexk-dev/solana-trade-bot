use crate::entity::{LimitOrder, LimitOrderType};
use anyhow::Result;
use async_trait::async_trait;
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, Message, ParseMode},
    Bot,
};

#[async_trait]
pub trait LimitOrderView: Send + Sync {
    async fn display_limit_orders(&self, orders: Vec<LimitOrder>) -> Result<()>;
    async fn display_no_orders(&self) -> Result<()>;
    async fn prompt_for_order_type(&self) -> Result<()>;
    async fn prompt_for_token_address(&self, order_type: &LimitOrderType) -> Result<()>;
    async fn display_invalid_token_address(&self) -> Result<()>;
    async fn display_token_info(
        &self,
        order_type: &LimitOrderType,
        token_address: &str,
        token_symbol: &str,
        current_price_in_sol: f64,
        current_price_in_usdc: f64,
    ) -> Result<()>;
    async fn display_invalid_price_amount(&self, error_message: String) -> Result<()>;
    async fn prompt_for_confirmation(
        &self,
        order_type: &LimitOrderType,
        token_address: &str,
        token_symbol: &str,
        price_in_sol: f64,
        amount: f64,
        total_sol: f64,
    ) -> Result<()>;
    async fn display_order_creation_success(
        &self,
        order_type: &LimitOrderType,
        token_symbol: &str,
        price_in_sol: f64,
        amount: f64,
        order_id: i32,
    ) -> Result<()>;
    async fn display_order_creation_error(
        &self,
        order_type: &LimitOrderType,
        token_symbol: &str,
        error_message: String,
    ) -> Result<()>;
    async fn display_order_cancelled(&self) -> Result<()>;
    async fn display_error(&self, error_message: String) -> Result<()>;
}

pub struct TelegramLimitOrderView {
    bot: Bot,
    chat_id: ChatId,
}

impl TelegramLimitOrderView {
    pub fn new(bot: Bot, chat_id: ChatId) -> Self {
        Self { bot, chat_id }
    }
}

#[async_trait]
impl LimitOrderView for TelegramLimitOrderView {
    async fn display_limit_orders(&self, orders: Vec<LimitOrder>) -> Result<()> {
        if orders.is_empty() {
            return self.display_no_orders().await;
        }

        // Group orders by type
        let mut buy_orders: Vec<&LimitOrder> = Vec::new();
        let mut sell_orders: Vec<&LimitOrder> = Vec::new();

        for order in &orders {
            if order.order_type == "BUY" {
                buy_orders.push(order);
            } else if order.order_type == "SELL" {
                sell_orders.push(order);
            }
        }

        // Format message
        let mut message = "<b>Your Active Limit Orders</b>\n\n".to_string();

        // Add buy orders section
        if !buy_orders.is_empty() {
            message.push_str("<b>Buy Orders:</b>\n");
            for order in buy_orders {
                let price_diff = if let Some(current_price) = order.current_price_in_sol {
                    let diff_percent = ((current_price / order.price_in_sol) * 100.0) - 100f64;
                    format!(
                        " ({:.2}% {})",
                        diff_percent.abs(),
                        if diff_percent < 0.0 { "above" } else { "below" }
                    )
                } else {
                    "".to_string()
                };

                message.push_str(&format!(
                    "• <b>#{}</b>: {:.6} {} at {:.6} SOL{}\n",
                    order.id, order.amount, order.token_symbol, order.price_in_sol, price_diff
                ));
            }
            message.push_str("\n");
        }

        // Add sell orders section
        if !sell_orders.is_empty() {
            message.push_str("<b>Sell Orders:</b>\n");
            for order in sell_orders {
                let price_diff = if let Some(current_price) = order.current_price_in_sol {
                    let diff_percent = ((current_price / order.price_in_sol) * 100.0) - 100f64;
                    format!(
                        " ({:.2}% {})",
                        diff_percent.abs(),
                        if diff_percent < 0.0 { "below" } else { "above" }
                    )
                } else {
                    "".to_string()
                };

                message.push_str(&format!(
                    "• <b>#{}</b>: {:.6} {} at {:.6} SOL{}\n",
                    order.id, order.amount, order.token_symbol, order.price_in_sol, price_diff
                ));
            }
            message.push_str("\n");
        }

        // Create keyboard with buttons
        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![
                InlineKeyboardButton::callback("Create Limit Order", "create_limit_order"),
                InlineKeyboardButton::callback("Back to Menu", "menu"),
            ],
            vec![
                InlineKeyboardButton::callback("Cancel Order", "cancel_limit_order"),
                InlineKeyboardButton::callback("🔄 Refresh", "refresh_limit_orders"),
            ],
        ]);

        // Send message with keyboard
        self.bot
            .send_message(self.chat_id, message)
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await?;

        Ok(())
    }

    async fn display_no_orders(&self) -> Result<()> {
        let message = "You don't have any active limit orders.";

        // Create keyboard with buttons
        let keyboard = InlineKeyboardMarkup::new(vec![vec![
            InlineKeyboardButton::callback("Create Limit Order", "create_limit_order"),
            InlineKeyboardButton::callback("Back to Menu", "menu"),
        ]]);

        // Send message with keyboard
        self.bot
            .send_message(self.chat_id, message)
            .reply_markup(keyboard)
            .await?;

        Ok(())
    }

    async fn prompt_for_order_type(&self) -> Result<()> {
        let message = "What type of limit order would you like to create?";

        // Create keyboard with buttons
        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![
                InlineKeyboardButton::callback("Limit Buy Order", "limit_buy_order"),
                InlineKeyboardButton::callback("Limit Sell Order", "limit_sell_order"),
            ],
            vec![InlineKeyboardButton::callback("Back to Menu", "menu")],
        ]);

        // Send message with keyboard
        self.bot
            .send_message(self.chat_id, message)
            .reply_markup(keyboard)
            .await?;

        Ok(())
    }

    async fn prompt_for_token_address(&self, order_type: &LimitOrderType) -> Result<()> {
        let action = match order_type {
            LimitOrderType::Buy => "buy",
            LimitOrderType::Sell => "sell",
        };

        self.bot
            .send_message(
                self.chat_id,
                format!(
                    "Please enter the token contract address you want to {}:",
                    action
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
        order_type: &LimitOrderType,
        token_address: &str,
        token_symbol: &str,
        current_price_in_sol: f64,
        current_price_in_usdc: f64,
    ) -> Result<()> {
        let action = match order_type {
            LimitOrderType::Buy => "buy",
            LimitOrderType::Sell => "sell",
        };

        self.bot
            .send_message(
                self.chat_id,
                format!(
                    "Token: {} ({})\nCurrent price: {:.6} SOL (${:.2})\n\nPlease enter the price in SOL and amount of tokens to {} in the format:\n<price> <amount>\n\nExample: 0.5 100",
                    token_symbol, token_address, current_price_in_sol, current_price_in_usdc, action
                ),
            )
            .await?;
        Ok(())
    }

    async fn display_invalid_price_amount(&self, error_message: String) -> Result<()> {
        self.bot
            .send_message(self.chat_id, format!("Error: {}", error_message))
            .await?;
        Ok(())
    }

    async fn prompt_for_confirmation(
        &self,
        order_type: &LimitOrderType,
        token_address: &str,
        token_symbol: &str,
        price_in_sol: f64,
        amount: f64,
        total_sol: f64,
    ) -> Result<()> {
        let order_type_str = match order_type {
            LimitOrderType::Buy => "BUY",
            LimitOrderType::Sell => "SELL",
        };

        self.bot
            .send_message(
                self.chat_id,
                format!(
                    "Please confirm your limit order:\n\n{} {} {} @ {:.6} SOL each\nTotal: {:.6} SOL\n\nDo you want to proceed? (yes/no)",
                    order_type_str, amount, token_symbol, price_in_sol, total_sol
                ),
            )
            .await?;
        Ok(())
    }

    async fn display_order_creation_success(
        &self,
        order_type: &LimitOrderType,
        token_symbol: &str,
        price_in_sol: f64,
        amount: f64,
        order_id: i32,
    ) -> Result<()> {
        let order_type_str = match order_type {
            LimitOrderType::Buy => "Buy",
            LimitOrderType::Sell => "Sell",
        };

        let keyboard = InlineKeyboardMarkup::new(vec![vec![
            InlineKeyboardButton::callback("View Orders", "limit_orders"),
            InlineKeyboardButton::callback("Back to Menu", "menu"),
        ]]);

        self.bot
            .send_message(
                self.chat_id,
                format!(
                    "✅ Limit {} Order #{} created successfully.\nAmount: {} {}\nPrice: {:.6} SOL per token\n\nYour order will execute when the market price reaches your specified price.",
                    order_type_str, order_id, amount, token_symbol, price_in_sol
                ),
            )
            .reply_markup(keyboard)
            .await?;
        Ok(())
    }

    async fn display_order_creation_error(
        &self,
        order_type: &LimitOrderType,
        token_symbol: &str,
        error_message: String,
    ) -> Result<()> {
        let order_type_str = match order_type {
            LimitOrderType::Buy => "buy",
            LimitOrderType::Sell => "sell",
        };

        self.bot
            .send_message(
                self.chat_id,
                format!(
                    "❌ Error creating limit {} order for {}:\n{}",
                    order_type_str, token_symbol, error_message
                ),
            )
            .await?;
        Ok(())
    }

    async fn display_order_cancelled(&self) -> Result<()> {
        self.bot
            .send_message(self.chat_id, "Order creation cancelled.")
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
