// src/commands/help.rs
use super::CommandHandler;
use crate::di::ServiceContainer;
use crate::MyDialogue;
use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use sqlx::PgPool;
use std::sync::Arc;
use teloxide::prelude::*;

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
        _solana_client: Option<Arc<RpcClient>>,
        services: Arc<ServiceContainer>,
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
