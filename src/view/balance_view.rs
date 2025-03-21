use crate::entity::TokenBalance;
use anyhow::Result;
use async_trait::async_trait;
use teloxide::{
    prelude::*,
    types::{Message, ParseMode},
    Bot,
};

#[async_trait]
pub trait BalanceView: Send + Sync {
    async fn display_loading(&self) -> Result<Option<Message>>;
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
}

#[async_trait]
impl BalanceView for TelegramBalanceView {
    async fn display_loading(&self) -> Result<Option<Message>> {
        let message = self
            .bot
            .send_message(self.chat_id, "Fetching balance and token information...")
            .await?;

        Ok(Some(message.clone()))
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
        // Format balances with USD values
        let mut response = format!(
            "💰 <b>Wallet Balance {}</b>\n\n",
            Self::format_address(&address)
        );

        // Show SOL balance with USD
        let sol_usd = usd_values
            .iter()
            .find(|(symbol, _)| symbol == "SOL")
            .map(|(_, value)| *value)
            .unwrap_or(0.0);
        response.push_str(&format!(
            "• <b>SOL</b>: {:.6} (~${:.2})\n",
            sol_balance, sol_usd
        ));

        // Sort tokens by USD value (descending)
        let mut token_display: Vec<(TokenBalance, f64)> = token_balances
            .iter()
            .map(|token| {
                let usd = usd_values
                    .iter()
                    .find(|(sym, _)| sym == &token.symbol)
                    .map(|(_, val)| *val)
                    .unwrap_or(0.0);
                (token.clone(), usd)
            })
            .filter(|(token, _)| token.amount > 0.0) // Filter out zero balances
            .collect();

        token_display.sort_by(|(_, usd1), (_, usd2)| {
            usd2.partial_cmp(usd1).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Add token balances with USD values
        if !token_display.is_empty() {
            response.push_str("\n<b>SPL Tokens:</b>\n");
            for (token, usd) in token_display {
                if usd > 0.0 {
                    response.push_str(&format!(
                        "• <b>{}</b>: {:.6} (~${:.2})\n",
                        token.symbol, token.amount, usd
                    ));
                } else {
                    response.push_str(&format!("• <b>{}</b>: {:.6}\n", token.symbol, token.amount));
                }
            }
        }

        // Add total portfolio value
        if total_usd > 0.0 {
            response.push_str(&format!(
                "\n<b>Total Portfolio Value:</b> ~${:.2}",
                total_usd
            ));
        }

        // Update the status message with the balance info
        if let Some(msg) = message {
            self.bot
                .edit_message_text(self.chat_id, msg.id, response)
                .parse_mode(ParseMode::Html)
                .await?;
        } else {
            self.bot
                .send_message(self.chat_id, response)
                .parse_mode(ParseMode::Html)
                .await?;
        }

        Ok(())
    }

    async fn display_no_wallet(&self, message: Option<Message>) -> Result<()> {
        let text = "You don't have a wallet yet. Use /create_wallet to create a new wallet.";

        if let Some(msg) = message {
            self.bot
                .edit_message_text(self.chat_id, msg.id, text)
                .await?;
        } else {
            self.bot.send_message(self.chat_id, text).await?;
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
