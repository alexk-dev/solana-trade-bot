// src/commands/swap.rs
use anyhow::Result;
use log;
use sqlx::PgPool;
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use rust_decimal::prelude::ToPrimitive;
use teloxide::prelude::*;

use crate::db;
use crate::solana;
use crate::solana::jupiter::SwapService;
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

                    // Создаем сервис свопа
                    let mut swap_service = SwapService::new();

                    // Получаем котировку
                    match swap_service.get_swap_quote(amount, &source_token, &target_token, slippage).await {
                        Ok(quote) => {
                            // Получаем информацию о целевом токене
                            let target_token_info = match swap_service.token_service.token_repository.get_token_by_id(&target_token).await {
                                Ok(token) => token,
                                Err(_) => {
                                    bot.edit_message_text(
                                        msg.chat.id,
                                        processing_msg.id,
                                        format!("❌ Ошибка при получении информации о токене {}", target_token)
                                    ).await?;
                                    return Ok(());
                                }
                            };

                            let out_amount:f64 = quote.out_amount.to_f64().unwrap();

                            // Применяем правильные decimals
                            let out_amount_float = out_amount / 10f64.powi(target_token_info.decimals as i32);

                            // Обновляем сообщение о прогрессе
                            bot.edit_message_text(
                                msg.chat.id,
                                processing_msg.id,
                                format!(
                                    "Котировка получена:\n\
                                    Вы отправите: {} {}\n\
                                    Получите: ~{:.6} {}\n\
                                    Проскальзывание: {}%\n\n\
                                    Получение транзакции от Jupiter API...",
                                    amount,
                                    source_token,
                                    out_amount_float,
                                    target_token,
                                    slippage * 100.0
                                )
                            ).await?;

                            // Подготавливаем и получаем транзакцию для свопа
                            match swap_service.prepare_swap(amount, &source_token, &target_token, slippage, &address).await {
                                Ok(swap_response) => {
                                    bot.edit_message_text(
                                        msg.chat.id,
                                        processing_msg.id,
                                        "Подписываем и отправляем транзакцию..."
                                    ).await?;

                                    // Получаем keypair из базы данных
                                    let keypair = solana::keypair_from_base58(&keypair_base58)?;

                                    // Выполняем свап (подписываем и отправляем транзакцию)
                                    match swap_service.execute_swap_transaction(&solana_client, &keypair, &swap_response).await {
                                        Ok(signature) => {
                                            // Запись транзакции в базу данных
                                            db::record_transaction(
                                                &db_pool,
                                                telegram_id,
                                                &target_token,
                                                out_amount_float,
                                                &target_token,
                                                &Some(signature.clone()),
                                                "SUCCESS"
                                            ).await?;

                                            // Отправляем пользователю информацию об успешном свопе
                                            bot.edit_message_text(
                                                msg.chat.id,
                                                processing_msg.id,
                                                format!(
                                                    "✅ Свап выполнен успешно!\n\
                                                    Отправлено: {} {}\n\
                                                    Получено: ~{:.6} {}\n\
                                                    Подпись транзакции: {}\n\
                                                    Проверить транзакцию: https://explorer.solana.com/tx/{}",
                                                    amount,
                                                    source_token,
                                                    out_amount_float,
                                                    target_token,
                                                    signature,
                                                    signature
                                                )
                                            ).await?;
                                        },
                                        Err(e) => {
                                            log::error!("Error sending swap transaction: {:?}", e);

                                            // Запись неудачной транзакции
                                            db::record_transaction(
                                                &db_pool,
                                                telegram_id,
                                                &target_token,
                                                out_amount_float,
                                                &target_token,
                                                &None::<String>,
                                                "FAILED"
                                            ).await?;

                                            // Информируем пользователя об ошибке
                                            bot.edit_message_text(
                                                msg.chat.id,
                                                processing_msg.id,
                                                format!(
                                                    "❌ Ошибка при отправке транзакции свопа:\n{}\n\n\
                                                    Возможные причины:\n\
                                                    - Недостаточно средств для комиссии\n\
                                                    - Проблемы с сетью Solana\n\
                                                    - Транзакция отклонена сетью",
                                                    e
                                                )
                                            ).await?;
                                        }
                                    }
                                },
                                Err(e) => {
                                    // Более подробный вывод ошибки
                                    log::error!("Swap transaction error: {:?}", e);
                                    bot.edit_message_text(
                                        msg.chat.id,
                                        processing_msg.id,
                                        format!(
                                            "❌ Ошибка при создании транзакции обмена:\n{}\n\n\
                                            Возможные причины:\n\
                                            - Неверный формат токенов\n\
                                            - Недостаточно средств\n\
                                            - Проблемы с подключением к API",
                                            e
                                        )
                                    ).await?;
                                }
                            }
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