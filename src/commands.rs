use anyhow::Result;
use log::{info, error, debug};
use sqlx::PgPool;
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use teloxide::{
    prelude::*,
    dispatching::{dialogue, UpdateHandler},
    types::ParseMode,
    utils::command::BotCommands,
};
use teloxide::dispatching::dialogue::InMemStorage;

use crate::{
    db,
    solana,
    solana::jupiter::TokenService,
    utils,
    model::{State, SwapParams, BotError},
    MyDialogue,
    qrcodeutils
};

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum Command {
    #[command(description = "start the bot")]
    Start,
    #[command(rename = "create_wallet", description = "create a new Solana wallet")]
    CreateWallet,
    #[command(description = "show your wallet address and QR code")]
    Address,
    #[command(description = "check your wallet balance")]
    Balance,
    #[command(description = "send funds to another address")]
    Send,
    #[command(description = "swap tokens via Raydium (format: /swap amount from_token to_token slippage%)")]
    Swap,
    #[command(description = "get price for a token")]
    Price,
    #[command(description = "display this help message")]
    Help,
}

pub fn setup_command_handlers() -> UpdateHandler<anyhow::Error> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(case![Command::Start].endpoint(start))
        .branch(case![Command::CreateWallet].endpoint(create_wallet))
        .branch(case![Command::Address].endpoint(address))
        .branch(case![Command::Balance].endpoint(balance))
        .branch(case![Command::Send].endpoint(send_start))
        .branch(case![Command::Swap].endpoint(swap))
        .branch(case![Command::Price].endpoint(price))
        .branch(case![Command::Help].endpoint(help));

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(dptree::entry()
            .branch(case![State::AwaitingRecipientAddress].endpoint(receive_recipient_address))
            .branch(case![State::AwaitingAmount { recipient }].endpoint(receive_amount))
            .branch(case![State::AwaitingConfirmation { recipient, amount, token }].endpoint(receive_confirmation))
            .branch(case![State::AwaitingSwapDetails].endpoint(receive_swap_details))
        );

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(message_handler)
}

//-----------------------------------------------------------------------------------------------
// START & REGISTER
//-----------------------------------------------------------------------------------------------
async fn start(bot: Bot, msg: Message, db_pool: PgPool) -> Result<()> {
    let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);
    let username = msg.from().and_then(|user| user.username.clone());

    info!("Start command received from Telegram ID: {}", telegram_id);

    let user_exists = db::check_user_exists(&db_pool, telegram_id)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;

    if !user_exists {
        db::create_user(&db_pool, telegram_id, username)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create user: {}", e))?;

        bot.send_message(
            msg.chat.id,
            "Привет! Я бот для управления Solana-кошельком. Вы успешно зарегистрированы.\n\n\
            Используйте /create_wallet чтобы создать новый кошелек, или /help для просмотра всех команд."
        )
            .parse_mode(ParseMode::Markdown)
            .await?;
    } else {
        bot.send_message(
            msg.chat.id,
            "С возвращением! Используйте /help для просмотра доступных команд."
        )
            .parse_mode(ParseMode::Markdown)
            .await?;
    }

    Ok(())
}

async fn register(bot: Bot, msg: Message, db_pool: PgPool) -> Result<()> {
    let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);
    let username = msg.from().and_then(|user| user.username.clone());

    info!("Register command received from Telegram ID: {}", telegram_id);

    let user_exists = db::check_user_exists(&db_pool, telegram_id)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;

    if !user_exists {
        db::create_user(&db_pool, telegram_id, username)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create user: {}", e))?;

        bot.send_message(
            msg.chat.id,
            "Вы успешно зарегистрированы! Используйте /create_wallet чтобы создать новый кошелек."
        )
            .parse_mode(ParseMode::Markdown)
            .await?;
    } else {
        bot.send_message(
            msg.chat.id,
            "Вы уже зарегистрированы. Используйте /help для просмотра доступных команд."
        )
            .parse_mode(ParseMode::Markdown)
            .await?;
    }

    Ok(())
}

//-----------------------------------------------------------------------------------------------
// CREATE WALLET, ADDRESS, BALANCE
//-----------------------------------------------------------------------------------------------
async fn create_wallet(bot: Bot, msg: Message, db_pool: PgPool) -> Result<()> {
    let telegram_id = msg.from.map_or(0, |user| user.id.0 as i64);

    info!("Create wallet command received from Telegram ID: {}", telegram_id);

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
        )
    )
        .parse_mode(ParseMode::Markdown)
        .await?;

    Ok(())
}

async fn address(bot: Bot, msg: Message, db_pool: PgPool) -> Result<()> {
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
            format!("Адрес вашего Solana-кошелька:\n\n`{}`", address)
        )
            .parse_mode(ParseMode::Markdown)
            .await?;

        // Send QR code as photo
        use teloxide::types::InputFile;
        let png_data: Vec<u8> = qrcodeutils::convert_svg_to_png(&qr_svg_data)?;

        bot.send_photo(
            msg.chat.id,
            InputFile::memory(png_data).file_name("address.png")
        )
            .caption("QR-код для вашего адреса")
            .await?;

    } else {
        bot.send_message(
            msg.chat.id,
            "У вас еще нет кошелька. Используйте /create_wallet чтобы создать новый кошелек."
        )
            .await?;
    }

    Ok(())
}

async fn balance(bot: Bot, msg: Message, db_pool: PgPool, solana_client: Arc<RpcClient>) -> Result<()> {
    let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);

    info!("Balance command received from Telegram ID: {}", telegram_id);

    // Get user's wallet address
    let user = db::get_user_by_telegram_id(&db_pool, telegram_id).await?;

    if let Some(address) = user.solana_address {
        // Send a status message
        let status_message = bot.send_message(
            msg.chat.id,
            "Получение информации о балансе..."
        ).await?;

        // Get SOL balance
        let sol_balance = solana::get_sol_balance(&solana_client, &address).await?;

        // Get token balances
        let token_balances = solana::get_token_balances(&solana_client, &address).await?;

        // Prepare response message
        let mut response = format!("Баланс вашего кошелька:\n\nSOL: {:.6}", sol_balance);

        if !token_balances.is_empty() {
            for token in token_balances {
                response.push_str(&format!("\n{}: {:.6}", token.symbol, token.amount));
            }
        }

        // Update the status message with the balance info
        bot.edit_message_text(msg.chat.id, status_message.id, response)
            .parse_mode(ParseMode::Markdown)
            .await?;
    } else {
        bot.send_message(
            msg.chat.id,
            "У вас еще нет кошелька. Используйте /create_wallet чтобы создать новый кошелек."
        )
            .await?;
    }

    Ok(())
}

//-----------------------------------------------------------------------------------------------
// SEND FLOW
//-----------------------------------------------------------------------------------------------
async fn send_start(bot: Bot, msg: Message, dialogue: MyDialogue) -> Result<()> {
    info!("Send command initiated");

    dialogue.update(State::AwaitingRecipientAddress).await?;

    bot.send_message(msg.chat.id, "Введите Solana-адрес получателя:").await?;

    Ok(())
}

async fn receive_recipient_address(bot: Bot, msg: Message, dialogue: MyDialogue) -> Result<()> {
    if let Some(address_text) = msg.text() {
        // Validate the address format
        if utils::validate_solana_address(address_text) {
            dialogue.update(State::AwaitingAmount { recipient: address_text.to_string() }).await?;

            bot.send_message(
                msg.chat.id,
                "Введите сумму для отправки (например: 0.5 SOL или 100 USDC):"
            ).await?;
        } else {
            bot.send_message(
                msg.chat.id,
                "Некорректный Solana-адрес. Пожалуйста, проверьте адрес и попробуйте снова:"
            ).await?;
        }
    } else {
        bot.send_message(
            msg.chat.id,
            "Пожалуйста, введите текстовый адрес получателя:"
        ).await?;
    }

    Ok(())
}

async fn receive_amount(
    bot: Bot,
    msg: Message,
    state: State,
    dialogue: MyDialogue
) -> Result<()> {
    if let State::AwaitingAmount { recipient } = state {
        if let Some(amount_text) = msg.text() {
            // Parse amount and token from the input
            if let Some((amount, token)) = utils::parse_amount_and_token(amount_text) {
                dialogue.update(State::AwaitingConfirmation {
                    recipient: recipient.clone(),
                    amount,
                    token: token.to_string()
                }).await?;

                bot.send_message(
                    msg.chat.id,
                    format!(
                        "Подтвердите отправку {} {} на адрес {} (да/нет):",
                        amount, token, recipient
                    )
                ).await?;
            } else {
                bot.send_message(
                    msg.chat.id,
                    "Некорректный формат суммы. Пожалуйста, введите в формате '0.5 SOL' или '100 USDC':"
                ).await?;
            }
        } else {
            bot.send_message(
                msg.chat.id,
                "Пожалуйста, введите сумму для отправки:"
            ).await?;
        }
    }

    Ok(())
}

async fn receive_confirmation(
    bot: Bot,
    msg: Message,
    state: State,
    dialogue: MyDialogue,
    db_pool: PgPool,
    solana_client: Arc<RpcClient>
) -> Result<()> {
    if let State::AwaitingConfirmation { recipient, amount, token } = state {
        if let Some(text) = msg.text() {
            let confirmation = text.to_lowercase();

            if confirmation == "да" || confirmation == "yes" {
                let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);

                // Reset dialogue state
                dialogue.update(State::Start).await?;

                // Send "processing" message
                let processing_msg = bot.send_message(
                    msg.chat.id,
                    "Отправка средств... Пожалуйста, подождите."
                ).await?;

                // Get user wallet info
                let user = db::get_user_by_telegram_id(&db_pool, telegram_id).await?;

                match user.solana_address {
                    Some(sender_address) => {
                        // Get private key
                        if let Some(keypair_base58) = user.encrypted_private_key {
                            let keypair = solana::keypair_from_base58(&keypair_base58)?;

                            // Send transaction
                            let result = if token.to_uppercase() == "SOL" {
                                solana::send_sol(
                                    &solana_client,
                                    &keypair,
                                    &recipient,
                                    amount
                                ).await
                            } else {
                                solana::send_spl_token(
                                    &solana_client,
                                    &keypair,
                                    &recipient,
                                    &token,
                                    amount
                                ).await
                            };

                            match result {
                                Ok(signature) => {
                                    // Record transaction to database
                                    db::record_transaction(
                                        &db_pool,
                                        telegram_id,
                                        &recipient,
                                        amount,
                                        &token,
                                        &Some(signature.clone()),
                                        "SUCCESS"
                                    ).await?;

                                    // Send success message
                                    bot.edit_message_text(
                                        msg.chat.id,
                                        processing_msg.id,
                                        format!("✅ Средства отправлены. Tx Signature: {}", signature)
                                    ).await?;
                                },
                                Err(e) => {
                                    error!("Failed to send transaction: {}", e);

                                    // Record failed transaction
                                    db::record_transaction(
                                        &db_pool,
                                        telegram_id,
                                        &recipient,
                                        amount,
                                        &token,
                                        &None::<String>,
                                        "FAILED"
                                    ).await?;

                                    // Send error message
                                    bot.edit_message_text(
                                        msg.chat.id,
                                        processing_msg.id,
                                        format!("❌ Ошибка при отправке средств: {}", e)
                                    ).await?;
                                }
                            }
                        } else {
                            bot.edit_message_text(
                                msg.chat.id,
                                processing_msg.id,
                                "❌ Ошибка: Не найден закрытый ключ для вашего кошелька."
                            ).await?;
                        }
                    },
                    None => {
                        bot.edit_message_text(
                            msg.chat.id,
                            processing_msg.id,
                            "❌ У вас еще нет кошелька. Используйте /create_wallet чтобы создать новый кошелек."
                        ).await?;
                    }
                }
            } else {
                // Transaction cancelled
                dialogue.update(State::Start).await?;

                bot.send_message(
                    msg.chat.id,
                    "Отправка средств отменена."
                ).await?;
            }
        }
    }

    Ok(())
}

//-----------------------------------------------------------------------------------------------
// SWAP COMMAND
//-----------------------------------------------------------------------------------------------
async fn swap(bot: Bot, msg: Message, db_pool: PgPool, solana_client: Arc<RpcClient>) -> Result<()> {
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

//-----------------------------------------------------------------------------------------------
// PRICE COMMAND
//-----------------------------------------------------------------------------------------------
async fn price(bot: Bot, msg: Message) -> Result<()> {
    let command_parts: Vec<&str> = msg.text().unwrap_or("").split_whitespace().collect();

    if command_parts.len() >= 2 {
        let token = command_parts[1];

        let processing_msg = bot.send_message(
            msg.chat.id,
            format!("Получение цены для {}...", token)
        ).await?;

        let mut token_service = TokenService::new();
        match token_service.get_token_price(&token).await {
            Ok(price_info) => {
                // price_info — это структура TokenPrice
                // Чтобы вывести её в текст, обращаемся к нужным полям,
                // например price_in_usdc или price_in_sol
                bot.edit_message_text(
                    msg.chat.id,
                    processing_msg.id,
                    format!(
                        "Текущая цена {}:\n≈ {:.6} SOL\n≈ {:.6} USDC",
                        token,
                        price_info.price_in_sol,
                        price_info.price_in_usdc,
                    )
                ).await?;
            },
            Err(e) => {
                bot.edit_message_text(
                    msg.chat.id,
                    processing_msg.id,
                    format!("❌ Ошибка при получении цены: {}", e)
                ).await?;
            }
        }
    } else {
        bot.send_message(
            msg.chat.id,
            "Используйте команду в формате: /price <символ_токена>\n\nПример: /price SOL"
        ).await?;
    }

    Ok(())
}

//-----------------------------------------------------------------------------------------------
// HELP
//-----------------------------------------------------------------------------------------------
async fn help(bot: Bot, msg: Message) -> Result<()> {
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

//-----------------------------------------------------------------------------------------------
// RECEIVE SWAP DETAILS (PLACEHOLDER)
//-----------------------------------------------------------------------------------------------
async fn receive_swap_details(bot: Bot, msg: Message, dialogue: MyDialogue) -> Result<()> {
    // Это заглушка, если вы хотели бы продолжить логику свопа через цепочку сообщений
    dialogue.update(State::Start).await?;
    bot.send_message(msg.chat.id, "Функция обмена токенов в разработке (placeholder).").await?;
    Ok(())
}