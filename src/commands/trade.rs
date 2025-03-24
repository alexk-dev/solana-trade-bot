use super::{CommandHandler, MyDialogue};
use crate::db;
use crate::di::ServiceContainer;
use crate::entity::State;
use crate::interactor::trade_interactor::{TradeInteractor, TradeInteractorImpl};
use crate::presenter::trade_presenter::{TradePresenter, TradePresenterImpl};
use crate::view::trade_view::TelegramTradeView;
use anyhow::Result;
use log::info;
use std::sync::Arc;
use teloxide::prelude::*;

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

        dialogue
            .update(State::AwaitingTokenAddress {
                trade_type: "BUY".to_string(),
            })
            .await?;

        let db_pool = services.db_pool();
        let solana_client = services.solana_client();
        let price_service = services.price_service();
        let token_repository = services.token_repository();
        let swap_service = services.swap_service();

        let interactor = Arc::new(TradeInteractorImpl::new(
            db_pool,
            solana_client,
            price_service,
            token_repository,
            swap_service,
        ));
        let view = Arc::new(TelegramTradeView::new(bot, chat_id));
        let presenter = TradePresenterImpl::new(interactor, view);

        presenter.start_trade_flow("BUY").await?;

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

        dialogue
            .update(State::AwaitingTokenAddress {
                trade_type: "SELL".to_string(),
            })
            .await?;

        let db_pool = services.db_pool();
        let solana_client = services.solana_client();
        let price_service = services.price_service();
        let token_repository = services.token_repository();
        let swap_service = services.swap_service();

        let interactor = Arc::new(TradeInteractorImpl::new(
            db_pool,
            solana_client,
            price_service,
            token_repository,
            swap_service,
        ));
        let view = Arc::new(TelegramTradeView::new(bot, chat_id));
        let presenter = TradePresenterImpl::new(interactor, view);

        presenter.start_trade_flow("SELL").await?;

        Ok(())
    }
}

// Handler for the token address state
pub async fn receive_token_address(
    bot: Bot,
    msg: Message,
    state: State,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    if let State::AwaitingTokenAddress { trade_type } = state {
        if let Some(address_text) = msg.text() {
            let chat_id = msg.chat.id;
            let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);
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
            let view = Arc::new(TelegramTradeView::new(bot.clone(), chat_id));
            let presenter = TradePresenterImpl::new(interactor.clone(), view);

            // Validate token address
            if let Ok(is_valid) = interactor.validate_token_address(address_text).await {
                if is_valid {
                    // Get token info to show to the user
                    match interactor.get_token_info(address_text).await {
                        Ok((token_symbol, price_in_sol, price_in_usdc)) => {
                            // For sell actions, get the user's token balance
                            if trade_type == "SELL" {
                                // Get user wallet address
                                match db::get_user_by_telegram_id(&db_pool, telegram_id).await {
                                    Ok(user) => {
                                        if let Some(user_address) = user.solana_address {
                                            // Get user's token balance
                                            match interactor
                                                .get_token_balance(address_text, &user_address)
                                                .await
                                            {
                                                Ok(token_balance) => {
                                                    // Update dialogue state
                                                    dialogue
                                                        .update(State::AwaitingTradeAmount {
                                                            trade_type: trade_type.clone(),
                                                            token_address: address_text.to_string(),
                                                            token_symbol: token_symbol.clone(),
                                                            price_in_sol,
                                                            price_in_usdc,
                                                        })
                                                        .await?;

                                                    // Display token info with balance
                                                    bot.send_message(
                                                        chat_id,
                                                        format!(
                                                            "Token: {} ({})\nCurrent price: {:.6} SOL (${:.2})\nYour balance: {} {}\n\nHow many tokens do you want to sell?\nType 'All' to sell your entire balance.",
                                                            token_symbol, address_text, price_in_sol, price_in_usdc, token_balance, token_symbol
                                                        ),
                                                    )
                                                        .await?;
                                                }
                                                Err(e) => {
                                                    bot.send_message(
                                                        chat_id,
                                                        format!(
                                                            "Error getting token balance: {}",
                                                            e
                                                        ),
                                                    )
                                                    .await?;
                                                }
                                            }
                                        } else {
                                            bot.send_message(
                                                chat_id,
                                                "You don't have a wallet yet. Use /create_wallet to create one.",
                                            )
                                                .await?;
                                        }
                                    }
                                    Err(e) => {
                                        bot.send_message(
                                            chat_id,
                                            format!("Error accessing user information: {}", e),
                                        )
                                        .await?;
                                    }
                                }
                            } else {
                                // For BUY actions, proceed normally
                                // Update dialogue state
                                dialogue
                                    .update(State::AwaitingTradeAmount {
                                        trade_type: trade_type.clone(),
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
                                        "Token: {} ({})\nCurrent price: {:.6} SOL (${:.2})\n\nHow many tokens do you want to {}?",
                                        token_symbol, address_text, price_in_sol, price_in_usdc, trade_type.to_lowercase()
                                    ),
                                )
                                    .await?;
                            }
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
    }

    Ok(())
}

// Handler for the trade amount state
pub async fn receive_trade_amount(
    bot: Bot,
    msg: Message,
    state: State,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    if let State::AwaitingTradeAmount {
        trade_type,
        token_address,
        token_symbol,
        price_in_sol,
        price_in_usdc,
    } = state
    {
        if let Some(amount_text) = msg.text() {
            let chat_id = msg.chat.id;
            let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);
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

            // Handle amount validation differently for buy vs sell
            if trade_type == "SELL" {
                // Get user's address for balance check
                match db::get_user_by_telegram_id(&db_pool, telegram_id).await {
                    Ok(user) => {
                        if let Some(user_address) = user.solana_address {
                            // Validate sell amount (includes handling "All" keyword)
                            match interactor
                                .validate_sell_amount(amount_text, &token_address, &user_address)
                                .await
                            {
                                Ok(amount) => {
                                    // Calculate total
                                    let total_sol = amount * price_in_sol;

                                    // Update dialogue state
                                    dialogue
                                        .update(State::AwaitingTradeConfirmation {
                                            trade_type: trade_type.clone(),
                                            token_address: token_address.clone(),
                                            token_symbol: token_symbol.clone(),
                                            amount,
                                            price_in_sol,
                                            total_sol,
                                        })
                                        .await?;

                                    // Prompt for confirmation
                                    bot.send_message(
                                        chat_id,
                                        format!(
                                            "Please confirm your trade:\n\n{} {} {}\nPrice per token: {:.6} SOL\nTotal: {:.6} SOL\n\nDo you want to proceed? (yes/no)",
                                            trade_type, amount, token_symbol, price_in_sol, total_sol
                                        ),
                                    )
                                        .await?;
                                }
                                Err(e) => {
                                    bot.send_message(chat_id, e.to_string()).await?;
                                }
                            }
                        } else {
                            bot.send_message(
                                chat_id,
                                "You don't have a wallet yet. Use /create_wallet to create one.",
                            )
                            .await?;
                        }
                    }
                    Err(e) => {
                        bot.send_message(
                            chat_id,
                            format!("Error accessing user information: {}", e),
                        )
                        .await?;
                    }
                }
            } else {
                // For BUY operations - standard validation
                match interactor.validate_buy_amount(amount_text).await {
                    Ok(amount) => {
                        // Calculate total
                        let total_sol = amount * price_in_sol;

                        // Update dialogue state
                        dialogue
                            .update(State::AwaitingTradeConfirmation {
                                trade_type: trade_type.clone(),
                                token_address: token_address.clone(),
                                token_symbol: token_symbol.clone(),
                                amount,
                                price_in_sol,
                                total_sol,
                            })
                            .await?;

                        // Prompt for confirmation
                        bot.send_message(
                            chat_id,
                            format!(
                                "Please confirm your trade:\n\n{} {} {}\nPrice per token: {:.6} SOL\nTotal: {:.6} SOL\n\nDo you want to proceed? (yes/no)",
                                trade_type, amount, token_symbol, price_in_sol, total_sol
                            ),
                        )
                            .await?;
                    }
                    Err(e) => {
                        bot.send_message(chat_id, e.to_string()).await?;
                    }
                }
            }
        } else {
            bot.send_message(msg.chat.id, "Please enter an amount as a number:")
                .await?;
        }
    }

    Ok(())
}

// Handler for the trade confirmation state
pub async fn receive_trade_confirmation(
    bot: Bot,
    msg: Message,
    state: State,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    if let State::AwaitingTradeConfirmation {
        trade_type,
        token_address,
        token_symbol,
        amount,
        price_in_sol,
        total_sol,
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
                        format!("Processing your {} order... Please wait.", trade_type),
                    )
                    .await?;

                // Execute the trade
                let db_pool = services.db_pool();
                let solana_client = services.solana_client();
                let price_service = services.price_service();
                let token_repository = services.token_repository();
                let swap_service = services.swap_service();

                let interactor = Arc::new(TradeInteractorImpl::new(
                    db_pool,
                    solana_client,
                    price_service,
                    token_repository,
                    swap_service,
                ));

                let result = interactor
                    .execute_trade(
                        telegram_id,
                        &trade_type,
                        &token_address,
                        &token_symbol,
                        amount,
                        price_in_sol,
                    )
                    .await?;

                if result.success {
                    // Trade was successful
                    let success_text = format!(
                        "✅ {} order completed successfully.\nAmount: {} {}\nPrice: {:.6} SOL per token\nTotal: {:.6} SOL\nTx Signature: {}\nCheck transaction: https://explorer.solana.com/tx/{}",
                        trade_type,
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
                        "❌ Error executing {} order for {} {}:\n{}",
                        trade_type,
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
