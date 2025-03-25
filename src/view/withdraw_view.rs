use crate::entity::TokenBalance;
use anyhow::Result;
use async_trait::async_trait;
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, Message, ParseMode},
    Bot,
};

#[async_trait]
pub trait WithdrawView: Send + Sync {
    async fn display_token_selection(&self, tokens: Vec<TokenBalance>) -> Result<()>;
    async fn display_token_details(
        &self,
        token_symbol: &str,
        token_address: &str,
        balance: f64,
        price_in_sol: f64,
        price_in_usdc: f64,
    ) -> Result<()>;
    async fn prompt_for_recipient_address(&self) -> Result<()>;
    async fn display_invalid_address(&self) -> Result<()>;
    async fn prompt_for_amount(
        &self,
        token_symbol: &str,
        balance: f64,
        price_in_sol: f64,
        price_in_usdc: f64,
    ) -> Result<()>;
    async fn display_invalid_amount(&self, error_message: String) -> Result<()>;
    async fn prompt_for_confirmation(
        &self,
        token_symbol: &str,
        recipient: &str,
        amount: f64,
        total_sol: f64,
        total_usdc: f64,
    ) -> Result<()>;
    async fn display_processing(&self) -> Result<Option<Message>>;
    async fn display_transaction_success(
        &self,
        token_symbol: &str,
        recipient: &str,
        amount: f64,
        signature: &str,
        message: Option<Message>,
    ) -> Result<()>;
    async fn display_transaction_error(
        &self,
        token_symbol: &str,
        recipient: &str,
        amount: f64,
        error_message: String,
        message: Option<Message>,
    ) -> Result<()>;
    async fn display_transaction_cancelled(&self) -> Result<()>;
    async fn display_no_tokens(&self) -> Result<()>;
    async fn display_no_wallet(&self) -> Result<()>;
    async fn display_error(&self, error_message: String) -> Result<()>;
}

pub struct TelegramWithdrawView {
    bot: Bot,
    chat_id: ChatId,
}

impl TelegramWithdrawView {
    pub fn new(bot: Bot, chat_id: ChatId) -> Self {
        Self { bot, chat_id }
    }
}

#[async_trait]
impl WithdrawView for TelegramWithdrawView {
    async fn display_token_selection(&self, tokens: Vec<TokenBalance>) -> Result<()> {
        if tokens.is_empty() {
            return self.display_no_tokens().await;
        }

        // Create keyboard buttons for each token
        let mut keyboard_buttons = Vec::new();

        for token in tokens {
            let token_text = format!("{}: {:.6}", token.symbol, token.amount);
            keyboard_buttons.push(vec![InlineKeyboardButton::callback(
                token_text,
                format!("withdraw_token_{}", token.mint_address),
            )]);
        }

        // Add cancel button
        keyboard_buttons.push(vec![InlineKeyboardButton::callback("← Cancel", "menu")]);

        let keyboard = InlineKeyboardMarkup::new(keyboard_buttons);

        self.bot
            .send_message(self.chat_id, "Select a token to withdraw:")
            .reply_markup(keyboard)
            .await?;

        Ok(())
    }

    async fn display_token_details(
        &self,
        token_symbol: &str,
        token_address: &str,
        balance: f64,
        price_in_sol: f64,
        price_in_usdc: f64,
    ) -> Result<()> {
        // Calculate total values
        let total_sol_value = balance * price_in_sol;
        let total_usdc_value = balance * price_in_usdc;

        // Format address for display (shortened)
        let short_address = if token_address.len() > 12 {
            format!(
                "{}...{}",
                &token_address[..6],
                &token_address[token_address.len() - 6..]
            )
        } else {
            token_address.to_string()
        };

        self.bot
            .send_message(
                self.chat_id,
                format!(
                    "<b>{} Token Details</b>\n\n\
                    • Symbol: <b>{}</b>\n\
                    • Address: <code>{}</code>\n\
                    • Your Balance: <b>{:.6}</b>\n\
                    • Price: <b>{:.6} SOL</b> (${:.2})\n\
                    • Total Value: <b>{:.6} SOL</b> (${:.2})\n\n\
                    Enter the recipient's Solana address:",
                    token_symbol,
                    token_symbol,
                    short_address,
                    balance,
                    price_in_sol,
                    price_in_usdc,
                    total_sol_value,
                    total_usdc_value
                ),
            )
            .parse_mode(ParseMode::Html)
            .await?;

        Ok(())
    }

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

    async fn prompt_for_amount(
        &self,
        token_symbol: &str,
        balance: f64,
        price_in_sol: f64,
        price_in_usdc: f64,
    ) -> Result<()> {
        self.bot
            .send_message(
                self.chat_id,
                format!(
                    "You have <b>{:.6} {}</b> (worth {:.6} SOL / ${:.2}).\n\n\
                    Enter the amount to withdraw:\n\
                    • Enter a specific amount (e.g. <code>0.5</code>)\n\
                    • Enter a percentage (e.g. <code>50%</code>)\n\
                    • Or type <code>All</code> to withdraw your entire balance",
                    balance,
                    token_symbol,
                    balance * price_in_sol,
                    balance * price_in_usdc
                ),
            )
            .parse_mode(ParseMode::Html)
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
        token_symbol: &str,
        recipient: &str,
        amount: f64,
        total_sol: f64,
        total_usdc: f64,
    ) -> Result<()> {
        // Format address for display (shortened)
        let short_address = if recipient.len() > 12 {
            format!(
                "{}...{}",
                &recipient[..6],
                &recipient[recipient.len() - 6..]
            )
        } else {
            recipient.to_string()
        };

        self.bot
            .send_message(
                self.chat_id,
                format!(
                    "<b>Confirm Withdrawal</b>\n\n\
                    • Amount: <b>{:.6} {}</b>\n\
                    • Value: <b>{:.6} SOL</b> (${:.2})\n\
                    • To: <code>{}</code>\n\n\
                    Proceed with this withdrawal? (yes/no)",
                    amount, token_symbol, total_sol, total_usdc, short_address
                ),
            )
            .parse_mode(ParseMode::Html)
            .await?;

        Ok(())
    }

    async fn display_processing(&self) -> Result<Option<Message>> {
        let message = self
            .bot
            .send_message(self.chat_id, "Processing withdrawal... Please wait.")
            .await?;

        Ok(Some(message))
    }

    async fn display_transaction_success(
        &self,
        token_symbol: &str,
        recipient: &str,
        amount: f64,
        signature: &str,
        message: Option<Message>,
    ) -> Result<()> {
        let text = format!(
            "✅ <b>Withdrawal Successful</b>\n\n\
            • Amount: <b>{:.6} {}</b>\n\
            • Recipient: <code>{}</code>\n\
            • Tx Signature: <code>{}</code>\n\n\
            <a href=\"https://explorer.solana.com/tx/{}\">View on Explorer</a>",
            amount, token_symbol, recipient, signature, signature
        );

        if let Some(msg) = message {
            self.bot
                .edit_message_text(self.chat_id, msg.id, text)
                .parse_mode(ParseMode::Html)
                .await?;
        } else {
            self.bot
                .send_message(self.chat_id, text)
                .parse_mode(ParseMode::Html)
                .await?;
        }

        Ok(())
    }

    async fn display_transaction_error(
        &self,
        token_symbol: &str,
        recipient: &str,
        amount: f64,
        error_message: String,
        message: Option<Message>,
    ) -> Result<()> {
        let text = format!(
            "❌ <b>Withdrawal Failed</b>\n\n\
            • Amount: <b>{:.6} {}</b>\n\
            • Recipient: <code>{}</code>\n\
            • Error: <code>{}</code>",
            amount, token_symbol, recipient, error_message
        );

        if let Some(msg) = message {
            self.bot
                .edit_message_text(self.chat_id, msg.id, text)
                .parse_mode(ParseMode::Html)
                .await?;
        } else {
            self.bot
                .send_message(self.chat_id, text)
                .parse_mode(ParseMode::Html)
                .await?;
        }

        Ok(())
    }

    async fn display_transaction_cancelled(&self) -> Result<()> {
        self.bot
            .send_message(self.chat_id, "Withdrawal cancelled.")
            .await?;

        Ok(())
    }

    async fn display_no_tokens(&self) -> Result<()> {
        self.bot
            .send_message(
                self.chat_id,
                "You don't have any tokens to withdraw. Deposit some tokens to your wallet first.",
            )
            .await?;

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

    async fn display_error(&self, error_message: String) -> Result<()> {
        self.bot
            .send_message(self.chat_id, format!("Error: {}", error_message))
            .await?;

        Ok(())
    }
}
