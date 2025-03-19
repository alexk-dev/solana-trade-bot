// src/commands/wallet.rs
use anyhow::Result;
use log::info;
use solana_client::nonblocking::rpc_client::RpcClient;
use sqlx::PgPool;
use std::sync::Arc;
use teloxide::{
    prelude::*,
    types::{InputFile, ParseMode},
};

use super::CommandHandler;
use crate::MyDialogue;
use crate::{db, qrcodeutils, solana, utils};

pub struct CreateWalletCommand;

impl CommandHandler for CreateWalletCommand {
    fn command_name() -> &'static str {
        "create_wallet"
    }

    fn description() -> &'static str {
        "create a new Solana wallet"
    }

    async fn execute(
        bot: Bot,
        msg: Message,
        _dialogue: Option<MyDialogue>,
        db_pool: Option<PgPool>,
        _solana_client: Option<Arc<RpcClient>>,
    ) -> Result<()> {
        let db_pool = db_pool.ok_or_else(|| anyhow::anyhow!("Database pool not provided"))?;
        let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);

        info!(
            "Create wallet command received from Telegram ID: {}",
            telegram_id
        );

        // Check if user already has a wallet
        let user = db::get_user_by_telegram_id(&db_pool, telegram_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get user: {}", e))?;

        if user.solana_address.is_some() {
            bot.send_message(
                msg.chat.id,
                "У вас уже есть кошелек Solana. Используйте /address чтобы увидеть адрес, или /balance для проверки баланса."
            )
                .await?;

            return Ok(());
        }

        // Generate new wallet
        let (mnemonic, keypair, address) = solana::generate_wallet()?;

        // Save wallet info to the database
        db::save_wallet_info(&db_pool, telegram_id, &address, &keypair, &mnemonic)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to save wallet info: {}", e))?;

        // Send wallet info to user
        bot.send_message(
            msg.chat.id,
            format!(
                "Ваш Solana-кошелёк успешно создан!\n\n\
                Публичный адрес: `{}`\n\n\
                Мнемоническая фраза: `{}`\n\n\
                *Важно:* Сохраните мнемоническую фразу – она нужна для восстановления доступа!",
                address, mnemonic
            ),
        )
        .parse_mode(ParseMode::Markdown)
        .await?;

        Ok(())
    }
}

pub struct AddressCommand;

impl CommandHandler for AddressCommand {
    fn command_name() -> &'static str {
        "address"
    }

    fn description() -> &'static str {
        "show your wallet address and QR code"
    }

    async fn execute(
        bot: Bot,
        msg: Message,
        _dialogue: Option<MyDialogue>,
        db_pool: Option<PgPool>,
        _solana_client: Option<Arc<RpcClient>>,
    ) -> Result<()> {
        let db_pool = db_pool.ok_or_else(|| anyhow::anyhow!("Database pool not provided"))?;
        let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);

        info!("Address command received from Telegram ID: {}", telegram_id);

        // Get user's wallet address
        let user = db::get_user_by_telegram_id(&db_pool, telegram_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get user: {}", e))?;

        if let Some(address) = user.solana_address {
            // Generate QR code
            let qr_svg_data = utils::generate_qr_code(&address)?;

            // Send address to user
            bot.send_message(
                msg.chat.id,
                format!("Адрес вашего Solana-кошелька:\n\n`{}`", address),
            )
            .parse_mode(ParseMode::Markdown)
            .await?;

            // Send QR code as photo
            let png_data: Vec<u8> = qrcodeutils::convert_svg_to_png(&qr_svg_data)?;

            bot.send_photo(
                msg.chat.id,
                InputFile::memory(png_data).file_name("address.png"),
            )
            .caption("QR-код для вашего адреса")
            .await?;
        } else {
            bot.send_message(
                msg.chat.id,
                "У вас еще нет кошелька. Используйте /create_wallet чтобы создать новый кошелек.",
            )
            .await?;
        }

        Ok(())
    }
}
