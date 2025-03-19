// src/commands/swap.rs
use anyhow::Result;
use sqlx::PgPool;
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use teloxide::prelude::*;

use crate::db;
use crate::solana::jupiter::TokenService;
use crate::model::State;
use crate::MyDialogue;
use super::CommandHandler;

pub struct SwapCommand;

impl CommandHandler for SwapCommand {
    fn command_name() -> &'static str {
        "swap"
    }

    fn description() -> &'static str {
        "swap tokens via Raydium (format: /swap amount from_token to_token slippage%)"
    }

    async fn execute(
        bot: Bot,
        msg: Message,
        _dialogue: Option<MyDialogue>,
        db_pool: Option<PgPool>,
        solana_client: Option<Arc<RpcClient>>
    ) -> Result<()> {
        let db_pool = db_pool.ok_or_else(|| anyhow::anyhow!("Database pool not provided"))?;
        let solana_client = solana_client.ok_or_else(|| anyhow::anyhow!("Solana client not provided"))?;
        let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);

        // Get full command text
        let command_parts: Vec<&str> = msg.text().unwrap_or("").split_whitespace().collect();

        if command_parts.len() >= 4 {
            // Parse swap parameters
            let amount_str = command_parts[1];
            let source_token = command_parts[2];
            let target_token = command_parts[3];

            // Parse slippage (optional)
            let slippage = if command_parts.len() >= 5
                && command_parts[4].ends_with('%')
                && command_parts[4].len() > 1 {
                command_parts[4]
                    .trim_end_matches('%')
                    .parse::<f64>()
                    .unwrap_or(0.5) / 100.0
            } else {
                0.005 // Default 0.5%
            };

            // Parse amount
            if let Ok(amount) = amount_str.parse::<f64>() {
                // Get user wallet info
                let user = db::get_user_by_telegram_id(&db_pool, telegram_id).await?;

                if let (Some(address), Some(keypair_base58)) = (user.solana_address, user.encrypted_private_key) {
                    // Отправляем «processing» сообщение
                    let processing_msg = bot.send_message(
                        msg.chat.id,
                        format!(
                            "Подготовка обмена {} {} на {}... Получение котировки...",
                            amount, source_token, target_token
                        )
                    ).await?;

                    match TokenService::new().get_swap_quote(amount, &source_token, &target_token, slippage).await {
                        Ok(quote) => {
                            // quote.out_amount (String) -> f64
                            let out_amount = quote
                                .out_amount
                                .parse::<f64>()
                                .unwrap_or(0.0);

                            // Для примера считаем, что это уже учтённые «мелкие единицы»
                            // или мы делим на 10^decimals в зависимости от логики.
                            // Допустим, здесь делим на 1e9 (как если бы это SOL).
                            let out_amount_float = out_amount / 1_000_000_000.0;

                            // Редактируем сообщение, показываем пользователю результат
                            bot.edit_message_text(
                                msg.chat.id,
                                processing_msg.id,
                                format!(
                                    "Котировка получена:\nВы отправите: {} {}\nПолучите: ~{:.6} {}\nПроскальзывание: {}%\n\n\
                                    (Заглушка: фактический свап не реализован.)",
                                    amount,
                                    source_token,
                                    out_amount_float,
                                    target_token,
                                    slippage * 100.0
                                )
                            ).await?;
                        },
                        Err(e) => {
                            bot.edit_message_text(
                                msg.chat.id,
                                processing_msg.id,
                                format!("❌ Ошибка при получении котировки: {}", e)
                            ).await?;
                        }
                    }
                } else {
                    bot.send_message(
                        msg.chat.id,
                        "❌ У вас еще нет кошелька. Используйте /create_wallet чтобы создать новый кошелек."
                    ).await?;
                }
            } else {
                bot.send_message(
                    msg.chat.id,
                    "❌ Некорректный формат суммы. Используйте: /swap 1.5 SOL USDC 0.5%"
                ).await?;
            }
        } else {
            // Show usage information
            bot.send_message(
                msg.chat.id,
                "Используйте команду в формате: /swap <сумма> <исходный_токен> <целевой_токен> [<проскальзывание>%]\n\n\
                 Пример: /swap 1.5 SOL USDC 0.5%"
            ).await?;
        }

        Ok(())
    }
}

pub async fn receive_swap_details(bot: Bot, msg: Message, dialogue: MyDialogue) -> Result<()> {
    // Это заглушка, если вы хотели бы продолжить логику свопа через цепочку сообщений
    dialogue.update(State::Start).await?;
    bot.send_message(msg.chat.id, "Функция обмена токенов в разработке (placeholder).").await?;
    Ok(())
}