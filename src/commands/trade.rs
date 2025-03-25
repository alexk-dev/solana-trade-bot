use super::{CommandHandler, MyDialogue};
use crate::di::ServiceContainer;
use crate::entity::{BotError, OrderType, State};
use crate::interactor::trade_interactor::{TradeInteractor, TradeInteractorImpl};
use crate::presenter::trade_presenter::{TradePresenter, TradePresenterImpl};
use crate::view::trade_view::TelegramTradeView;
use crate::{db, solana, TokenBalance};
use anyhow::Result;
use log::info;
use solana_client::nonblocking::rpc_client::RpcClient;
use sqlx::PgPool;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};

pub struct BuyCommand;

impl CommandHandler for BuyCommand {
    fn command_name() -> &'static str {
        "buy"
    }

    fn description() -> &'static str {
        "buy tokens on Solana"
    }

    async fn execute(
        bot: Bot,
        msg: Message,
        telegram_id: i64,
        dialogue: Option<MyDialogue>,
        services: Arc<ServiceContainer>,
    ) -> Result<()> {
        let dialogue = dialogue.ok_or_else(|| anyhow::anyhow!("Dialogue context not provided"))?;
        let chat_id = msg.chat.id;

        info!("Buy command initiated by user: {}", telegram_id);

        // Update dialogue state to token selection rather than directly asking for address
        dialogue.update(State::AwaitingBuyTokenSelection).await?;

        // Create set to track token addresses to avoid duplicates
        let mut token_addresses = std::collections::HashSet::new();
        let mut keyboard_buttons = Vec::new();

        // Step 1: Get user's existing tokens
        let db_pool = services.db_pool();
        let solana_client = services.solana_client();

        if let Ok(user_tokens) =
            get_user_tokens(telegram_id, db_pool.clone(), solana_client.clone()).await
        {
            for token in user_tokens {
                if token.symbol != "SOL" && token_addresses.insert(token.mint_address.clone()) {
                    let token_text = format!("{} (owned)", token.symbol);
                    keyboard_buttons.push(vec![InlineKeyboardButton::callback(
                        token_text,
                        format!("buy_token_{}", token.mint_address),
                    )]);
                }
            }
        }

        // Step 2: Get user's watchlist tokens
        if let Ok(watchlist) = db::get_user_watchlist(&db_pool, telegram_id).await {
            for item in watchlist {
                if token_addresses.insert(item.token_address.clone()) {
                    let token_text = format!("{} (watchlist)", item.token_symbol);
                    keyboard_buttons.push(vec![InlineKeyboardButton::callback(
                        token_text,
                        format!("buy_token_{}", item.token_address),
                    )]);
                }
            }
        }

        // Step 3: Add USDT and USDC from constants if not already added
        let usdt_address = crate::solana::tokens::constants::USDT_MINT;
        let usdc_address = crate::solana::tokens::constants::USDC_MINT;

        if token_addresses.insert(usdt_address.to_string()) {
            keyboard_buttons.push(vec![InlineKeyboardButton::callback(
                "USDT",
                format!("buy_token_{}", usdt_address),
            )]);
        }

        if token_addresses.insert(usdc_address.to_string()) {
            keyboard_buttons.push(vec![InlineKeyboardButton::callback(
                "USDC",
                format!("buy_token_{}", usdc_address),
            )]);
        }

        // Step 4: Add button for manual address entry
        keyboard_buttons.push(vec![InlineKeyboardButton::callback(
            "Enter Token Address Manually",
            "buy_manual_address",
        )]);

        // Add cancel button
        keyboard_buttons.push(vec![InlineKeyboardButton::callback("← Cancel", "menu")]);

        let keyboard = InlineKeyboardMarkup::new(keyboard_buttons);

        bot.send_message(
            chat_id,
            "Select a token to buy or enter a contract address manually:",
        )
        .reply_markup(keyboard)
        .await?;

        Ok(())
    }
}

pub struct SellCommand;

impl CommandHandler for SellCommand {
    fn command_name() -> &'static str {
        "sell"
    }

    fn description() -> &'static str {
        "sell tokens on Solana"
    }

    async fn execute(
        bot: Bot,
        msg: Message,
        telegram_id: i64,
        dialogue: Option<MyDialogue>,
        services: Arc<ServiceContainer>,
    ) -> Result<()> {
        let dialogue = dialogue.ok_or_else(|| anyhow::anyhow!("Dialogue context not provided"))?;
        let chat_id = msg.chat.id;

        info!("Sell command initiated by user: {}", telegram_id);

        // Update dialogue state to token selection rather than directly asking for address
        dialogue.update(State::AwaitingSellTokenSelection).await?;

        // Display token selection to user
        let db_pool = services.db_pool();
        let solana_client = services.solana_client();
        let price_service = services.price_service();

        // Get user's tokens
        match get_user_tokens(telegram_id, db_pool.clone(), solana_client.clone()).await {
            Ok(tokens) => {
                if tokens.is_empty() {
                    bot.send_message(
                        chat_id,
                        "You don't have any tokens to sell. Please deposit some tokens to your wallet first."
                    ).await?;
                } else {
                    // Create keyboard buttons for each token
                    let mut keyboard_buttons = Vec::new();

                    for token in tokens {
                        if token.symbol != "SOL" {
                            // Exclude SOL from the sell options
                            let token_text = format!("{}: {:.6}", token.symbol, token.amount);
                            keyboard_buttons.push(vec![InlineKeyboardButton::callback(
                                token_text,
                                format!("sell_token_{}", token.mint_address),
                            )]);
                        }
                    }

                    // Add cancel button
                    keyboard_buttons.push(vec![InlineKeyboardButton::callback("← Cancel", "menu")]);

                    let keyboard = InlineKeyboardMarkup::new(keyboard_buttons);

                    bot.send_message(chat_id, "Select a token to sell:")
                        .reply_markup(keyboard)
                        .await?;
                }
            }
            Err(e) => {
                if e.to_string().contains("Wallet not found") {
                    bot.send_message(
                        chat_id,
                        "You don't have a wallet yet. Use /create_wallet to create a new wallet.",
                    )
                    .await?;
                } else {
                    bot.send_message(chat_id, format!("Error retrieving tokens: {}", e))
                        .await?;
                }
            }
        }

        Ok(())
    }
}

// Helper function to get user tokens (similar to the one in withdraw)
pub(crate) async fn get_user_tokens(
    telegram_id: i64,
    db_pool: Arc<PgPool>,
    solana_client: Arc<RpcClient>,
) -> Result<Vec<TokenBalance>> {
    // Get user's wallet address
    let user = db::get_user_by_telegram_id(&db_pool, telegram_id).await?;

    let address = user
        .solana_address
        .ok_or_else(|| BotError::WalletNotFound)?;

    // Get token balances
    let token_balances = solana::get_token_balances(&solana_client, &address).await?;

    // Filter out zero balances
    let non_zero_balances = token_balances
        .into_iter()
        .filter(|balance| balance.amount > 0.0)
        .collect();

    Ok(non_zero_balances)
}

// New handler for sell amount input after token selection
pub async fn receive_sell_amount(
    bot: Bot,
    msg: Message,
    state: State,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    if let State::AwaitingSellAmount {
        token_address,
        token_symbol,
        balance,
        price_in_sol,
        price_in_usdc,
    } = state
    {
        if let Some(amount_text) = msg.text() {
            let chat_id = msg.chat.id;
            let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);

            // Create interactor for token operations
            let db_pool = services.db_pool();
            let solana_client = services.solana_client();
            let price_service = services.price_service();

            // Validate amount
            let amount = if amount_text.to_lowercase() == "all" {
                // User wants to sell all tokens
                balance
            } else if amount_text.ends_with('%') {
                // User specified a percentage
                let percentage_str = amount_text.trim_end_matches('%');
                match percentage_str.parse::<f64>() {
                    Ok(percentage) if percentage > 0.0 && percentage <= 100.0 => {
                        balance * (percentage / 100.0)
                    }
                    Ok(_) => {
                        bot.send_message(chat_id, "Percentage must be between 0 and 100%")
                            .await?;
                        return Ok(());
                    }
                    Err(_) => {
                        bot.send_message(
                            chat_id,
                            "Invalid percentage format. Please enter a number followed by %",
                        )
                        .await?;
                        return Ok(());
                    }
                }
            } else {
                // User specified a direct amount
                match amount_text.parse::<f64>() {
                    Ok(amount) if amount > 0.0 => {
                        if amount > balance {
                            bot.send_message(
                                chat_id,
                                format!("Insufficient balance. You only have {} tokens", balance),
                            )
                            .await?;
                            return Ok(());
                        }
                        amount
                    }
                    Ok(_) => {
                        bot.send_message(chat_id, "Amount must be greater than zero")
                            .await?;
                        return Ok(());
                    }
                    Err(_) => {
                        bot.send_message(
                            chat_id,
                            "Invalid amount format. Please enter a number, percentage, or 'All'",
                        )
                        .await?;
                        return Ok(());
                    }
                }
            };

            // Calculate total values
            let total_sol = amount * price_in_sol;
            let total_usdc = amount * price_in_usdc;

            // Update dialogue state
            dialogue
                .update(State::AwaitingSellConfirmation {
                    token_address: token_address.clone(),
                    token_symbol: token_symbol.clone(),
                    amount,
                    price_in_sol,
                    total_sol,
                    total_usdc,
                })
                .await?;

            // Prompt for confirmation
            bot.send_message(
                chat_id,
                format!(
                    "<b>Confirm Sell Order</b>\n\n\
                    • Sell: <b>{:.6} {}</b>\n\
                    • Price: <b>{:.6} SOL</b> per token\n\
                    • Total: <b>{:.6} SOL</b> (${:.2})\n\n\
                    Do you want to proceed? (yes/no)",
                    amount, token_symbol, price_in_sol, total_sol, total_usdc
                ),
            )
            .parse_mode(ParseMode::Html)
            .await?;
        } else {
            bot.send_message(msg.chat.id, "Please enter the amount as text:")
                .await?;
        }
    }

    Ok(())
}

// New handler for sell confirmation
pub async fn receive_sell_confirmation(
    bot: Bot,
    msg: Message,
    state: State,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    if let State::AwaitingSellConfirmation {
        token_address,
        token_symbol,
        amount,
        price_in_sol,
        total_sol,
        total_usdc,
    } = state
    {
        if let Some(text) = msg.text() {
            let confirmation = text.to_lowercase();
            let chat_id = msg.chat.id;
            let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);

            // Reset dialogue state
            dialogue.update(State::Start).await?;

            if confirmation == "yes" || confirmation == "y" {
                // Show processing message
                let processing_msg = bot
                    .send_message(
                        chat_id,
                        format!("Processing your SELL order... Please wait."),
                    )
                    .await?;

                // Execute the trade
                let db_pool = services.db_pool();
                let solana_client = services.solana_client();
                let price_service = services.price_service();
                let token_repository = services.token_repository();
                let swap_service = services.swap_service();

                let interactor = Arc::new(TradeInteractorImpl::new(
                    db_pool.clone(),
                    solana_client,
                    price_service,
                    token_repository,
                    swap_service,
                ));

                let result = interactor
                    .execute_trade(
                        telegram_id,
                        &OrderType::Sell,
                        &token_address,
                        &token_symbol,
                        amount,
                        price_in_sol,
                    )
                    .await?;

                if result.success {
                    // Trade was successful
                    let success_text = format!(
                        "✅ SELL order completed successfully.\n\
                        Amount: {} {}\n\
                        Price: {:.6} SOL per token\n\
                        Total: {:.6} SOL\n\
                        Tx Signature: {}\n\
                        Check transaction: https://explorer.solana.com/tx/{}",
                        amount,
                        token_symbol,
                        price_in_sol,
                        total_sol,
                        result.signature.as_deref().unwrap_or("unknown"),
                        result.signature.as_deref().unwrap_or("unknown")
                    );

                    bot.edit_message_text(chat_id, processing_msg.id, success_text)
                        .await?;
                } else {
                    // Trade failed
                    let error_text = format!(
                        "❌ Error executing SELL order for {} {}:\n{}",
                        amount,
                        token_symbol,
                        result
                            .error_message
                            .unwrap_or_else(|| "Unknown error".to_string())
                    );

                    bot.edit_message_text(chat_id, processing_msg.id, error_text)
                        .await?;
                }
            } else {
                // User cancelled the trade
                bot.send_message(chat_id, "Trade cancelled.").await?;
            }
        } else {
            bot.send_message(msg.chat.id, "Please confirm with 'yes' or 'no' as text:")
                .await?;
        }
    }

    Ok(())
}

// Handler for manual token address entry
pub async fn receive_buy_manual_address(
    bot: Bot,
    msg: Message,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    if let Some(address_text) = msg.text() {
        let chat_id = msg.chat.id;
        let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);

        // Validate the token address
        let db_pool = services.db_pool();
        let solana_client = services.solana_client();
        let price_service = services.price_service();
        let token_repository = services.token_repository();
        let swap_service = services.swap_service();

        let interactor = Arc::new(TradeInteractorImpl::new(
            db_pool.clone(),
            solana_client.clone(),
            price_service.clone(),
            token_repository.clone(),
            swap_service.clone(),
        ));

        if let Ok(is_valid) = interactor.validate_token_address(address_text).await {
            if is_valid {
                // Get token info to display to the user
                match interactor.get_token_info(address_text).await {
                    Ok((token_symbol, price_in_sol, price_in_usdc)) => {
                        // Update dialogue state
                        dialogue
                            .update(State::AwaitingBuyAmount {
                                token_address: address_text.to_string(),
                                token_symbol: token_symbol.clone(),
                                price_in_sol,
                                price_in_usdc,
                            })
                            .await?;

                        // Display token info
                        bot.send_message(
                            chat_id,
                            format!(
                                "Token: {} ({})\nCurrent price: {:.6} SOL (${:.2})\n\nHow many tokens do you want to buy?",
                                token_symbol, address_text, price_in_sol, price_in_usdc
                            ),
                        )
                            .await?;
                    }
                    Err(e) => {
                        bot.send_message(chat_id, format!("Error getting token info: {}", e))
                            .await?;
                    }
                }
            } else {
                bot.send_message(
                    chat_id,
                    "Invalid token address. Please enter a valid Solana token contract address:",
                )
                .await?;
            }
        } else {
            bot.send_message(chat_id, "Error validating token address. Please try again:")
                .await?;
        }
    } else {
        bot.send_message(
            msg.chat.id,
            "Please enter the token contract address as text:",
        )
        .await?;
    }

    Ok(())
}

// Handler for buy amount
pub async fn receive_buy_amount(
    bot: Bot,
    msg: Message,
    state: State,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    if let State::AwaitingBuyAmount {
        token_address,
        token_symbol,
        price_in_sol,
        price_in_usdc,
    } = state
    {
        if let Some(amount_text) = msg.text() {
            let chat_id = msg.chat.id;

            // Validate amount
            match amount_text.parse::<f64>() {
                Ok(amount) if amount > 0.0 => {
                    // Calculate total
                    let total_sol = amount * price_in_sol;
                    let total_usdc = amount * price_in_usdc;

                    // Update dialogue state
                    dialogue
                        .update(State::AwaitingBuyConfirmation {
                            token_address: token_address.clone(),
                            token_symbol: token_symbol.clone(),
                            amount,
                            price_in_sol,
                            total_sol,
                            total_usdc,
                        })
                        .await?;

                    // Prompt for confirmation
                    bot.send_message(
                        chat_id,
                        format!(
                            "<b>Confirm Buy Order</b>\n\n\
                            • Buy: <b>{:.6} {}</b>\n\
                            • Price: <b>{:.6} SOL</b> per token\n\
                            • Total: <b>{:.6} SOL</b> (${:.2})\n\n\
                            Do you want to proceed? (yes/no)",
                            amount, token_symbol, price_in_sol, total_sol, total_usdc
                        ),
                    )
                    .parse_mode(ParseMode::Html)
                    .await?;
                }
                Ok(_) => {
                    bot.send_message(chat_id, "Amount must be greater than zero")
                        .await?;
                }
                Err(_) => {
                    bot.send_message(chat_id, "Invalid amount format. Please enter a number.")
                        .await?;
                }
            }
        } else {
            bot.send_message(msg.chat.id, "Please enter the amount as text:")
                .await?;
        }
    }

    Ok(())
}

// Handler for buy confirmation
pub async fn receive_buy_confirmation(
    bot: Bot,
    msg: Message,
    state: State,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    if let State::AwaitingBuyConfirmation {
        token_address,
        token_symbol,
        amount,
        price_in_sol,
        total_sol,
        total_usdc,
    } = state
    {
        if let Some(text) = msg.text() {
            let confirmation = text.to_lowercase();
            let chat_id = msg.chat.id;
            let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);

            // Reset dialogue state
            dialogue.update(State::Start).await?;

            if confirmation == "yes" || confirmation == "y" {
                // Show processing message
                let processing_msg = bot
                    .send_message(
                        chat_id,
                        format!("Processing your BUY order... Please wait."),
                    )
                    .await?;

                // Execute the trade
                let db_pool = services.db_pool();
                let solana_client = services.solana_client();
                let price_service = services.price_service();
                let token_repository = services.token_repository();
                let swap_service = services.swap_service();

                let interactor = Arc::new(TradeInteractorImpl::new(
                    db_pool.clone(),
                    solana_client,
                    price_service,
                    token_repository,
                    swap_service,
                ));

                let result = interactor
                    .execute_trade(
                        telegram_id,
                        &OrderType::Buy,
                        &token_address,
                        &token_symbol,
                        amount,
                        price_in_sol,
                    )
                    .await?;

                if result.success {
                    // Trade was successful
                    let success_text = format!(
                        "✅ BUY order completed successfully.\n\
                        Amount: {} {}\n\
                        Price: {:.6} SOL per token\n\
                        Total: {:.6} SOL\n\
                        Tx Signature: {}\n\
                        Check transaction: https://explorer.solana.com/tx/{}",
                        amount,
                        token_symbol,
                        price_in_sol,
                        total_sol,
                        result.signature.as_deref().unwrap_or("unknown"),
                        result.signature.as_deref().unwrap_or("unknown")
                    );

                    bot.edit_message_text(chat_id, processing_msg.id, success_text)
                        .await?;
                } else {
                    // Trade failed
                    let error_text = format!(
                        "❌ Error executing BUY order for {} {}:\n{}",
                        amount,
                        token_symbol,
                        result
                            .error_message
                            .unwrap_or_else(|| "Unknown error".to_string())
                    );

                    bot.edit_message_text(chat_id, processing_msg.id, error_text)
                        .await?;
                }
            } else {
                // User cancelled the trade
                bot.send_message(chat_id, "Trade cancelled.").await?;
            }
        } else {
            bot.send_message(msg.chat.id, "Please confirm with 'yes' or 'no' as text:")
                .await?;
        }
    }

    Ok(())
}
