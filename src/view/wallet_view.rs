use crate::qrcodeutils;
use crate::utils;
use anyhow::Result;
use async_trait::async_trait;
use teloxide::{
    prelude::*,
    types::{InputFile, ParseMode},
    Bot,
};

#[async_trait]
pub trait WalletView: Send + Sync {
    async fn display_wallet_created(&self, address: String, mnemonic: String) -> Result<()>;
    async fn display_wallet_address(&self, address: String) -> Result<()>;
    async fn display_no_wallet(&self) -> Result<()>;
    async fn display_wallet_already_exists(&self) -> Result<()>;
    async fn display_error(&self, error_message: String) -> Result<()>;
}

pub struct TelegramWalletView {
    bot: Bot,
    chat_id: ChatId,
}

impl TelegramWalletView {
    pub fn new(bot: Bot, chat_id: ChatId) -> Self {
        Self { bot, chat_id }
    }
}

#[async_trait]
impl WalletView for TelegramWalletView {
    async fn display_wallet_created(&self, address: String, mnemonic: String) -> Result<()> {
        // Send wallet info to user
        self.bot
            .send_message(
                self.chat_id,
                std::format!(
                    "Your Solana wallet has been successfully created!\n\n\
                Public address: `{}`\n\n\
                Mnemonic phrase: `{}`\n\n\
                <b>Important:</b> Save your mnemonic phrase - it's needed to recover access!",
                    address,
                    mnemonic
                ),
            )
            .parse_mode(ParseMode::Html)
            .await?;

        Ok(())
    }

    async fn display_wallet_address(&self, address: String) -> Result<()> {
        // Generate QR code
        let qr_svg_data = utils::generate_qr_code(&address)?;

        // Send address to user
        self.bot
            .send_message(
                self.chat_id,
                format!("Your Solana wallet address:\n\n <b>{}</b>", address),
            )
            .parse_mode(ParseMode::Html)
            .await?;

        // Send QR code as photo
        let png_data: Vec<u8> = qrcodeutils::convert_svg_to_png(&qr_svg_data)?;

        self.bot
            .send_photo(
                self.chat_id,
                InputFile::memory(png_data).file_name("address.png"),
            )
            .caption("QR code for your address")
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

    async fn display_wallet_already_exists(&self) -> Result<()> {
        self.bot.send_message(
            self.chat_id,
            "You already have a Solana wallet. Use /address to see the address, or /balance to check your balance."
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
