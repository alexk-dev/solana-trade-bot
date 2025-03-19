// src/commands/help.rs
use anyhow::Result;
use sqlx::PgPool;
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use teloxide::prelude::*;

use crate::MyDialogue;
use super::CommandHandler;

pub struct HelpCommand;

impl CommandHandler for HelpCommand {
    fn command_name() -> &'static str {
        "help"
    }

    fn description() -> &'static str {
        "display this help message"
    }

    async fn execute(
        bot: Bot,
        msg: Message,
        _dialogue: Option<MyDialogue>,
        _db_pool: Option<PgPool>,
        _solana_client: Option<Arc<RpcClient>>
    ) -> Result<()> {
        bot.send_message(
            msg.chat.id,
            "Доступные команды:\n\
            /start - Начать работу с ботом\n\
            /create_wallet - Создать новый кошелек Solana\n\
            /address - Показать адрес вашего кошелька и QR-код\n\
            /balance - Проверить баланс вашего кошелька\n\
            /send - Отправить средства на другой адрес\n\
            /swap <сумма> <исходный_токен> <целевой_токен> [<проскальзывание>%] - Обменять токены через Raydium DEX (заглушка)\n\
            /price <символ_токена> - Получить текущую цену токена\n\
            /help - Показать эту справку"
        ).await?;

        Ok(())
    }
}