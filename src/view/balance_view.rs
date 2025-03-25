use crate::commands::ui;
use crate::entity::TokenBalance;
use anyhow::Result;
use async_trait::async_trait;
use chrono;
use teloxide::{
    prelude::*,
    types::{Message, ParseMode},
    Bot,
};

#[async_trait]
pub trait BalanceView: Send + Sync {
    async fn display_loading(&self) -> Result<Option<Message>>;
    async fn display_loading_update(&self, message: Message) -> Result<Option<Message>>;
    async fn display_balances(
        &self,
        address: String,
        sol_balance: f64,
        token_balances: Vec<TokenBalance>,
        usd_values: Vec<(String, f64)>,
        total_usd: f64,
        message: Option<Message>,
    ) -> Result<()>;

    async fn display_no_wallet(&self, message: Option<Message>) -> Result<()>;
    async fn display_error(&self, error_message: String, message: Option<Message>) -> Result<()>;
}

pub struct TelegramBalanceView {
    bot: Bot,
    chat_id: ChatId,
}

impl TelegramBalanceView {
    pub fn new(bot: Bot, chat_id: ChatId) -> Self {
        Self { bot, chat_id }
    }

    // Helper function to format wallet address
    fn format_address(address: &str) -> String {
        if address.len() <= 12 {
            return address.to_string();
        }
        format!("{}...{}", &address[..6], &address[address.len() - 4..])
    }

    fn format_total_portfolio_text(&self, total_usd: f64) -> String {
        // Add total portfolio value
        if total_usd > 0.0 {
            let text = format!("<b>Total Portfolio Value:</b> ${:.2}", total_usd);

            return text;
        }

        String::new()
    }

    fn format_spl_tokens_text(
        &self,
        token_balances: &Vec<TokenBalance>,
        usd_values: &Vec<(String, f64)>,
    ) -> String {
        // If there are token balances, display them in a separate message
        if !token_balances.is_empty() {
            let mut tokens_text = "\n\n<b>Token Balances</b>\n\n".to_string();
            let mut any_token_gt_zero = false;

            for token in token_balances {
                if token.amount > 0.0 {
                    any_token_gt_zero = true;
                    // Get USD value for this token
                    let token_usd = usd_values
                        .iter()
                        .find(|(symbol, _)| symbol == &token.symbol)
                        .map(|(_, value)| *value)
                        .unwrap_or(0.0);

                    if token_usd > 0.0 {
                        tokens_text.push_str(&format!(
                            "• <b>{}</b>: {:.6} (${:.2})\n",
                            token.symbol, token.amount, token_usd
                        ));
                    } else {
                        tokens_text
                            .push_str(&format!("• <b>{}</b>: {:.6}\n", token.symbol, token.amount));
                    }
                }
            }

            if (!any_token_gt_zero) {
                return String::new();
            }

            return tokens_text;
        }

        String::new()
    }
}

#[async_trait]
impl BalanceView for TelegramBalanceView {
    async fn display_loading(&self) -> Result<Option<Message>> {
        let message = self
            .bot
            .send_message(self.chat_id, "Fetching balance and token information...")
            .await?;

        Ok(Some(message))
    }

    async fn display_loading_update(&self, message: Message) -> Result<Option<Message>> {
        let updated_msg = self
            .bot
            .edit_message_text(
                self.chat_id,
                message.id,
                "Refreshing balance information...",
            )
            .await?;

        Ok(Some(updated_msg))
    }

    async fn display_balances(
        &self,
        address: String,
        sol_balance: f64,
        token_balances: Vec<TokenBalance>,
        usd_values: Vec<(String, f64)>,
        total_usd: f64,
        message: Option<Message>,
    ) -> Result<()> {
        // Get SOL price in USD from the usd_values array
        let sol_usd_value = usd_values
            .iter()
            .find(|(symbol, _)| symbol == "SOL")
            .map(|(_, value)| *value)
            .unwrap_or(0.0);

        // Calculate SOL price by dividing the USD value by the balance (if balance > 0)
        let sol_price = if sol_balance > 0.0 {
            sol_usd_value / sol_balance
        } else {
            0.0
        };

        let sol_text = format!(
            "<b>Solana</b> · 🔑\n\
            <code>{}</code>\n\n\
            Balance: <b>{:.6}</b> SOL (${:.2})",
            address, sol_balance, sol_usd_value
        );

        let token_text = self.format_spl_tokens_text(&token_balances, &usd_values);

        let portfolio_total = self.format_total_portfolio_text(total_usd);

        let updated_text = format!(
            "—\n\n\
            Updated: {} UTC",
            chrono::Utc::now().format("%H:%M:%S")
        );

        let text = sol_text
            + token_text.as_str()
            + "\n\n"
            + portfolio_total.as_str()
            + "\n\n"
            + updated_text.as_str();

        // Get the keyboard from UI module
        let keyboard = ui::create_wallet_menu_keyboard();

        // Update existing message or send a new one
        if let Some(msg) = message {
            self.bot
                .edit_message_text(self.chat_id, msg.id, text)
                .parse_mode(ParseMode::Html)
                .reply_markup(keyboard)
                .await?;
        } else {
            self.bot
                .send_message(self.chat_id, text)
                .parse_mode(ParseMode::Html)
                .reply_markup(keyboard)
                .await?;
        }

        Ok(())
    }

    async fn display_no_wallet(&self, message: Option<Message>) -> Result<()> {
        let text = "You don't have a wallet yet. Use /create_wallet to create a new wallet.";
        let keyboard = ui::create_wallet_menu_keyboard();

        if let Some(msg) = message {
            self.bot
                .edit_message_text(self.chat_id, msg.id, text)
                .reply_markup(keyboard)
                .await?;
        } else {
            self.bot
                .send_message(self.chat_id, text)
                .reply_markup(keyboard)
                .await?;
        }

        Ok(())
    }

    async fn display_error(&self, error_message: String, message: Option<Message>) -> Result<()> {
        let text = format!("Error: {}", error_message);

        if let Some(msg) = message {
            self.bot
                .edit_message_text(self.chat_id, msg.id, text)
                .await?;
        } else {
            self.bot.send_message(self.chat_id, text).await?;
        }

        Ok(())
    }
}
