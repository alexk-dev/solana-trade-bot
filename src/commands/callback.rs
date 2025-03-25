use anyhow::Result;
use log::info;
use std::{str::FromStr, sync::Arc};
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode},
};

use crate::commands::{help, price, trade, ui, wallet, CommandHandler, MyDialogue};
use crate::db;
use crate::di::ServiceContainer;
use crate::entity::State;
use crate::interactor::balance_interactor::{BalanceInteractor, BalanceInteractorImpl};
use crate::interactor::trade_interactor::{TradeInteractor, TradeInteractorImpl};
use crate::interactor::wallet_interactor::WalletInteractorImpl;
use crate::interactor::withdraw_interactor::WithdrawInteractor;
use crate::presenter::balance_presenter::{BalancePresenter, BalancePresenterImpl};
use crate::presenter::limit_order_presenter::LimitOrderPresenter;
use crate::presenter::settings_presenter::SettingsPresenter;
use crate::presenter::watchlist_presenter::WatchlistPresenter;
use crate::presenter::withdraw_presenter::WithdrawPresenter;
use crate::view::balance_view::TelegramBalanceView;

// Main callback handler function
pub async fn handle_callback(
    bot: Bot,
    q: CallbackQuery,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    // Extract the callback data
    let callback_data = match q.clone().data {
        Some(data) => data,
        None => return Ok(()),
    };

    let message = q.regular_message().unwrap();

    // Get the chat ID
    let chat_id = match q.message {
        Some(ref msg) => msg.chat().id,
        None => return Ok(()),
    };

    // Get user's Telegram ID
    let telegram_id = q.from.id.0 as i64;

    info!(
        "Received callback: {} from user {}",
        callback_data, telegram_id
    );

    // Acknowledge the callback query to stop loading animation
    if let Err(err) = bot.answer_callback_query(q.id.clone()).await {
        info!("Failed to answer callback query: {}", err);
    }

    // Process the callback based on its type
    if callback_data == ("menu") || callback_data == "refresh" {
        // Handle refresh action - update balance display
        handle_refresh(&bot, Some(message.clone()), telegram_id, services).await?;
    } else if callback_data == "create_wallet" {
        // Handle create wallet action
        if let msg = message.clone() {
            wallet::CreateWalletCommand::execute(bot, msg, telegram_id, Some(dialogue), services)
                .await?;
        }
    } else if callback_data == "address" {
        // Handle address action
        if let msg = message.clone() {
            wallet::AddressCommand::execute(bot, msg, telegram_id, Some(dialogue), services)
                .await?;
        }
    } else if callback_data == "price" {
        // Handle price action - show token selection
        handle_check_price(&bot, chat_id, dialogue).await?;
    } else if callback_data.starts_with("price_") {
        // Handle specific token price request
        handle_price_selection(&bot, &callback_data, chat_id, services).await?;
    } else if callback_data == "help" {
        // Handle help action
        if let msg = message.clone() {
            help::HelpCommand::execute(bot, msg, telegram_id, Some(dialogue), services).await?;
        }
    } else if callback_data == "buy" {
        // Handle buy action - show token selection
        handle_buy_start(&bot, message.clone(), telegram_id, dialogue, services).await?;
    } else if callback_data == "buy_manual_address" {
        // Handle manual address entry for buy
        handle_buy_manual_address(&bot, message.clone(), dialogue).await?;
    } else if callback_data.starts_with("buy_token_") {
        // Handle token selection for buy
        let token_address = callback_data.strip_prefix("buy_token_").unwrap_or("");
        handle_buy_token_selection(
            &bot,
            token_address,
            message.clone(),
            telegram_id,
            dialogue,
            services,
        )
        .await?;
    } else if callback_data == "sell" {
        // Handle sell action - show token selection
        handle_sell_start(&bot, message.clone(), telegram_id, dialogue, services).await?;
    } else if callback_data.starts_with("sell_token_") {
        // Handle token selection for sell
        let token_address = callback_data.strip_prefix("sell_token_").unwrap_or("");
        handle_sell_token_selection(
            &bot,
            token_address,
            message.clone(),
            telegram_id,
            dialogue,
            services,
        )
        .await?;
    } else if callback_data == "limit_orders" {
        // Display limit orders
        handle_limit_orders(&bot, message.clone(), telegram_id, services).await?;
    } else if callback_data == "create_limit_order" {
        // Start limit order creation flow
        handle_create_limit_order(&bot, message.clone(), dialogue, services).await?;
    } else if callback_data == "limit_buy_order" {
        // Handle limit buy order type selection
        crate::commands::limit_order::handle_order_type_selection(
            bot,
            message.clone(),
            crate::entity::OrderType::Buy,
            dialogue,
            services,
        )
        .await?;
    } else if callback_data == "limit_sell_order" {
        // Handle limit sell order type selection
        crate::commands::limit_order::handle_order_type_selection(
            bot,
            message.clone(),
            crate::entity::OrderType::Sell,
            dialogue,
            services,
        )
        .await?;
    } else if callback_data == "refresh_limit_orders" {
        // Refresh limit orders display
        handle_limit_orders(&bot, message.clone(), telegram_id, services).await?;
    } else if callback_data == "cancel_limit_order" {
        // Show list of orders that can be cancelled
        handle_show_cancelable_orders(&bot, message.clone(), telegram_id, services).await?;
    } else if callback_data.starts_with("cancel_order_") {
        // Handle specific order cancellation
        let order_id_str = callback_data.strip_prefix("cancel_order_").unwrap_or("");
        if let Ok(order_id) = order_id_str.parse::<i32>() {
            handle_cancel_order(&bot, message.clone(), order_id, telegram_id, services).await?;
        } else {
            bot.send_message(chat_id, "Invalid order ID").await?;
        }
    } else if callback_data == "cancel_all_orders" {
        // Handle cancel all orders request
        handle_cancel_all_orders(&bot, message.clone(), telegram_id, services).await?;
    } else if callback_data == "confirm_cancel_all" {
        // Handle confirmation of cancelling all orders
        handle_confirm_cancel_all(&bot, message.clone(), telegram_id, services).await?;
    } else if callback_data == "settings" {
        // Handle settings menu action
        handle_settings_menu(&bot, message.clone(), telegram_id, services).await?;
    } else if callback_data == "set_slippage" {
        // Handle slippage setting action
        handle_set_slippage(&bot, message.clone(), dialogue, telegram_id, services).await?;
    } else if callback_data.starts_with("slippage_") {
        // Handle preset slippage values
        handle_preset_slippage(&bot, &callback_data, message.clone(), telegram_id, services)
            .await?;
    } else if callback_data == "watchlist" {
        // Handle watchlist menu
        handle_watchlist_menu(&bot, message.clone(), telegram_id, services).await?;
    } else if callback_data == "watchlist_add" {
        // Handle add to watchlist
        handle_watchlist_add(&bot, message.clone(), dialogue, telegram_id, services).await?;
    } else if callback_data == "watchlist_refresh" {
        // Handle watchlist refresh
        handle_watchlist_refresh(&bot, message.clone(), telegram_id, services).await?;
    } else if callback_data.starts_with("watchlist_view_") {
        // Handle view token details
        let token_address = callback_data.strip_prefix("watchlist_view_").unwrap_or("");
        handle_watchlist_view_token(&bot, token_address, message.clone(), telegram_id, services)
            .await?;
    } else if callback_data.starts_with("watchlist_remove_") {
        // Handle remove from watchlist
        let token_address = callback_data
            .strip_prefix("watchlist_remove_")
            .unwrap_or("");
        handle_watchlist_remove_token(&bot, token_address, message.clone(), telegram_id, services)
            .await?;
    } else if callback_data == "withdraw" {
        // Handle withdraw action - show token selection
        handle_withdraw_start(&bot, message.clone(), telegram_id, dialogue, services).await?;
    } else if callback_data.starts_with("withdraw_token_") {
        // Handle token selection for withdraw
        let token_address = callback_data.strip_prefix("withdraw_token_").unwrap_or("");
        handle_withdraw_token_selection(
            &bot,
            token_address,
            message.clone(),
            telegram_id,
            dialogue,
            services,
        )
        .await?;
    } else {
        // Handle trading UI buttons
        bot.send_message(
            chat_id,
            format!("The {} feature is under development.", callback_data),
        )
        .await?;
    }

    Ok(())
}

// Function to show token price selection
async fn handle_check_price(bot: &Bot, chat_id: ChatId, dialogue: MyDialogue) -> Result<()> {
    dialogue.update(State::AwaitingPriceTokenAddress).await?;

    // Prompt user for token address
    bot.send_message(
        chat_id,
        "Please enter the token contract address you want to check the price for:",
    )
    .await?;

    Ok(())
}

// Function to handle token price selection
async fn handle_price_selection(
    bot: &Bot,
    callback_data: &str,
    chat_id: ChatId,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let token = callback_data.strip_prefix("price_").unwrap_or("SOL");

    // Send loading message
    let message = bot
        .send_message(chat_id, format!("Getting price for {}...", token))
        .await?;

    // Call price service
    let price_service = services.price_service();

    match price_service.get_token_price(token).await {
        Ok(price_info) => {
            // Format price message
            let price_text = format!(
                "Current price for {}:\n≈ {:.6} SOL\n≈ ${:.6}",
                price_info.symbol, price_info.price_in_sol, price_info.price_in_usdc
            );

            // Add back button
            let keyboard = InlineKeyboardMarkup::new(vec![vec![
                InlineKeyboardButton::callback("Check Another Price", "price"),
                InlineKeyboardButton::callback("← Back to Menu", "menu"),
            ]]);

            // Update message with price info
            bot.edit_message_text(chat_id, message.id, price_text)
                .reply_markup(keyboard)
                .await?;
        }
        Err(e) => {
            // Show error message
            bot.edit_message_text(
                chat_id,
                message.id,
                format!("Error getting price for {}: {}", token, e),
            )
            .await?;
        }
    }

    Ok(())
}

// Function to handle refresh action
async fn handle_refresh(
    bot: &Bot,
    message: Option<Message>,
    telegram_id: i64,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    if let Some(msg) = message {
        let chat_id = msg.chat.id;

        // Create presenter to display refreshed information
        let solana_client = services.solana_client();
        let price_service = services.price_service();
        let interactor = Arc::new(BalanceInteractorImpl::new(
            services.db_pool(),
            solana_client,
            price_service,
        ));
        let view = Arc::new(TelegramBalanceView::new(bot.clone(), chat_id));
        let presenter = BalancePresenterImpl::new(interactor, view);

        // Call the refresh method that updates the existing message
        presenter.refresh_balances(telegram_id, Some(msg)).await?;
    }

    Ok(())
}

// Function to handle swap with predefined amount
async fn handle_swap_amount(
    bot: &Bot,
    callback_data: &str,
    chat_id: ChatId,
    telegram_id: i64,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    // Parse the callback data (format: swap_amount_AMOUNT_SOURCE_to_TARGET)
    let parts: Vec<&str> = callback_data.split('_').collect();

    if parts.len() >= 5 {
        let amount_str = parts[2];
        let source_token = parts[3];
        let target_token = parts[4];

        let amount = match f64::from_str(amount_str) {
            Ok(val) => val,
            Err(_) => {
                bot.send_message(chat_id, "Invalid amount format. Please try again.")
                    .await?;
                return Ok(());
            }
        };

        // Create confirmation keyboard
        let confirm_keyboard = InlineKeyboardMarkup::new(vec![vec![
            InlineKeyboardButton::callback(
                "✅ Confirm Swap",
                format!("confirm_swap_{}_{}_{}", amount, source_token, target_token),
            ),
            InlineKeyboardButton::callback("❌ Cancel", "swap"),
        ]]);

        // Show confirmation message
        bot.send_message(
            chat_id,
            format!(
                "You are about to swap {} {} to {}.\n\nDo you want to proceed?",
                amount, source_token, target_token
            ),
        )
        .reply_markup(confirm_keyboard)
        .await?;
    } else {
        bot.send_message(chat_id, "Invalid swap parameters. Please try again.")
            .await?;
    }

    Ok(())
}

// Function to display limit orders
async fn handle_limit_orders(
    bot: &Bot,
    message: Message,
    telegram_id: i64,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let chat_id = message.chat.id;

    // Create presenter to display limit orders
    let db_pool = services.db_pool();
    let solana_client = services.solana_client();
    let price_service = services.price_service();
    let token_repository = services.token_repository();

    let interactor = Arc::new(
        crate::interactor::limit_order_interactor::LimitOrderInteractorImpl::new(
            db_pool,
            solana_client,
            price_service,
            token_repository,
        ),
    );
    let view = Arc::new(crate::view::limit_order_view::TelegramLimitOrderView::new(
        bot.clone(),
        chat_id,
    ));
    let presenter =
        crate::presenter::limit_order_presenter::LimitOrderPresenterImpl::new(interactor, view);

    // Show limit orders
    presenter.show_limit_orders(telegram_id).await?;

    Ok(())
}

// Function to start limit order creation
async fn handle_create_limit_order(
    bot: &Bot,
    message: Message,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let chat_id = message.chat.id;

    // Update dialogue state
    dialogue
        .update(crate::entity::State::AwaitingLimitOrderType)
        .await?;

    // Create presenter for limit order creation
    let db_pool = services.db_pool();
    let solana_client = services.solana_client();
    let price_service = services.price_service();
    let token_repository = services.token_repository();

    let interactor = Arc::new(
        crate::interactor::limit_order_interactor::LimitOrderInteractorImpl::new(
            db_pool,
            solana_client,
            price_service,
            token_repository,
        ),
    );
    let view = Arc::new(crate::view::limit_order_view::TelegramLimitOrderView::new(
        bot.clone(),
        chat_id,
    ));
    let presenter =
        crate::presenter::limit_order_presenter::LimitOrderPresenterImpl::new(interactor, view);

    // Start limit order creation flow
    presenter.start_create_order_flow().await?;

    Ok(())
}

// Function to show cancelable orders
async fn handle_show_cancelable_orders(
    bot: &Bot,
    message: Message,
    telegram_id: i64,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let chat_id = message.chat.id;

    // Get active orders
    let db_pool = services.db_pool();
    let orders = crate::interactor::db::get_active_limit_orders(&db_pool, telegram_id).await?;

    if orders.is_empty() {
        bot.send_message(chat_id, "You don't have any active orders to cancel.")
            .await?;
        return Ok(());
    }

    // Create inline keyboard with cancel buttons for each order
    let mut keyboard_buttons = Vec::new();
    for order in &orders {
        let button_text = format!(
            "#{}: {} {} @ {} SOL",
            order.id, order.amount, order.token_symbol, order.price_in_sol
        );
        keyboard_buttons.push(vec![InlineKeyboardButton::callback(
            button_text,
            format!("cancel_order_{}", order.id),
        )]);
    }

    // Add back button
    keyboard_buttons.push(vec![InlineKeyboardButton::callback(
        "Back to Orders",
        "limit_orders",
    )]);

    let keyboard = InlineKeyboardMarkup::new(keyboard_buttons);

    // Send message with cancel options
    bot.send_message(chat_id, "Select an order to cancel:")
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

// Function to cancel a specific order
async fn handle_cancel_order(
    bot: &Bot,
    message: Message,
    order_id: i32,
    telegram_id: i64,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let db_pool = services.db_pool();

    // Verify order exists and belongs to user
    let user = crate::interactor::db::get_user_by_telegram_id(&db_pool, telegram_id).await?;
    let order = crate::interactor::db::get_limit_order_by_id(&db_pool, order_id).await?;

    match order {
        Some(order) if order.user_id == user.id => {
            // Cancel the order
            crate::interactor::db::cancel_limit_order(&db_pool, order_id).await?;

            // Send confirmation
            bot.send_message(
                ChatId(telegram_id),
                format!(
                    "Order #{} ({} {} @ {} SOL) has been cancelled.",
                    order_id, order.amount, order.token_symbol, order.price_in_sol
                ),
            )
            .await?;

            // Refresh orders list
            handle_limit_orders(bot, message, telegram_id, services).await?;
        }
        Some(_) => {
            // Order exists but doesn't belong to user
            bot.send_message(
                ChatId(telegram_id),
                "You don't have permission to cancel this order.",
            )
            .await?;
        }
        None => {
            // Order doesn't exist
            bot.send_message(
                ChatId(telegram_id),
                format!("Order #{} not found.", order_id),
            )
            .await?;
        }
    }

    Ok(())
}

// Function to cancel all orders
async fn handle_cancel_all_orders(
    bot: &Bot,
    message: Message,
    telegram_id: i64,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let chat_id = message.chat.id;
    let db_pool = services.db_pool();

    // First check if the user has any active orders
    let orders = crate::interactor::db::get_active_limit_orders(&db_pool, telegram_id).await?;

    if orders.is_empty() {
        // No active orders, just inform the user
        bot.send_message(chat_id, "You don't have any active orders to cancel.")
            .await?;
        return Ok(());
    }

    // Ask for confirmation
    let confirm_keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("Yes, Cancel All Orders", "confirm_cancel_all"),
        InlineKeyboardButton::callback("No, Keep My Orders", "limit_orders"),
    ]]);

    bot.send_message(
        chat_id,
        format!(
            "Are you sure you want to cancel all {} active limit orders?",
            orders.len()
        ),
    )
    .reply_markup(confirm_keyboard)
    .await?;

    Ok(())
}

// Function to handle confirmation of cancelling all orders
async fn handle_confirm_cancel_all(
    bot: &Bot,
    message: Message,
    telegram_id: i64,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let chat_id = message.chat.id;
    let db_pool = services.db_pool();

    // Cancel all active orders
    let cancelled_count =
        crate::interactor::db::cancel_all_limit_orders(&db_pool, telegram_id).await?;

    // Notify the user
    bot.send_message(
        chat_id,
        format!(
            "✅ Successfully cancelled {} limit orders.",
            cancelled_count
        ),
    )
    .await?;

    // Refresh the orders list
    handle_limit_orders(bot, message, telegram_id, services).await?;

    Ok(())
}

// Function to handle showing settings menu
async fn handle_settings_menu(
    bot: &Bot,
    message: Message,
    telegram_id: i64,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let chat_id = message.chat.id;

    // Create presenter for settings
    let db_pool = services.db_pool();
    let interactor =
        Arc::new(crate::interactor::settings_interactor::SettingsInteractorImpl::new(db_pool));
    let view = Arc::new(crate::view::settings_view::TelegramSettingsView::new(
        bot.clone(),
        chat_id,
    ));
    let presenter =
        crate::presenter::settings_presenter::SettingsPresenterImpl::new(interactor, view);

    // Show settings menu
    presenter.show_settings_menu(telegram_id).await?;

    Ok(())
}

// Function to handle slippage setting
async fn handle_set_slippage(
    bot: &Bot,
    message: Message,
    dialogue: MyDialogue,
    telegram_id: i64,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let chat_id = message.chat.id;

    // Update dialogue state to expect slippage input
    dialogue.update(State::AwaitingSlippageInput).await?;

    // Show slippage prompt
    let db_pool = services.db_pool();
    let interactor =
        Arc::new(crate::interactor::settings_interactor::SettingsInteractorImpl::new(db_pool));
    let view = Arc::new(crate::view::settings_view::TelegramSettingsView::new(
        bot.clone(),
        chat_id,
    ));
    let presenter =
        crate::presenter::settings_presenter::SettingsPresenterImpl::new(interactor, view);

    presenter.show_slippage_prompt(telegram_id).await?;

    Ok(())
}

// Function to handle preset slippage selections
async fn handle_preset_slippage(
    bot: &Bot,
    callback_data: &str,
    message: Message,
    telegram_id: i64,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let chat_id = message.chat.id;

    // Extract slippage value from callback data (format: "slippage_X.Y")
    let slippage_str = callback_data.strip_prefix("slippage_").unwrap_or("0.5");
    let slippage = slippage_str.parse::<f64>().unwrap_or(0.5);

    // Update slippage setting
    let db_pool = services.db_pool();
    let interactor =
        Arc::new(crate::interactor::settings_interactor::SettingsInteractorImpl::new(db_pool));
    let view = Arc::new(crate::view::settings_view::TelegramSettingsView::new(
        bot.clone(),
        chat_id,
    ));
    let presenter =
        crate::presenter::settings_presenter::SettingsPresenterImpl::new(interactor, view);

    presenter.set_preset_slippage(telegram_id, slippage).await?;

    Ok(())
}

// Function to show watchlist menu
async fn handle_watchlist_menu(
    bot: &Bot,
    message: Message,
    telegram_id: i64,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let chat_id = message.chat.id;

    // Create presenter for watchlist
    let db_pool = services.db_pool();
    let price_service = services.price_service();
    let token_repository = services.token_repository();

    let interactor = Arc::new(
        crate::interactor::watchlist_interactor::WatchlistInteractorImpl::new(
            db_pool,
            price_service.clone(),
            token_repository,
        ),
    );
    let view = Arc::new(crate::view::watchlist_view::TelegramWatchlistView::new(
        bot.clone(),
        chat_id,
    ));
    let presenter = crate::presenter::watchlist_presenter::WatchlistPresenterImpl::new(
        interactor,
        view,
        price_service,
    );

    // Show watchlist
    presenter.show_watchlist(telegram_id).await?;

    Ok(())
}

// Function to handle adding to watchlist
async fn handle_watchlist_add(
    bot: &Bot,
    message: Message,
    dialogue: MyDialogue,
    telegram_id: i64,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let chat_id = message.chat.id;

    // Update dialogue state to expect token address
    dialogue
        .update(State::AwaitingWatchlistTokenAddress)
        .await?;

    // Create presenter
    let db_pool = services.db_pool();
    let price_service = services.price_service();
    let token_repository = services.token_repository();

    let interactor = Arc::new(
        crate::interactor::watchlist_interactor::WatchlistInteractorImpl::new(
            db_pool,
            price_service.clone(),
            token_repository,
        ),
    );
    let view = Arc::new(crate::view::watchlist_view::TelegramWatchlistView::new(
        bot.clone(),
        chat_id,
    ));
    let presenter = crate::presenter::watchlist_presenter::WatchlistPresenterImpl::new(
        interactor,
        view,
        price_service,
    );

    // Prompt for token address
    presenter.prompt_for_token_address().await?;

    Ok(())
}

// Function to refresh watchlist prices
async fn handle_watchlist_refresh(
    bot: &Bot,
    message: Message,
    telegram_id: i64,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let chat_id = message.chat.id;

    // Create presenter
    let db_pool = services.db_pool();
    let price_service = services.price_service();
    let token_repository = services.token_repository();

    let interactor = Arc::new(
        crate::interactor::watchlist_interactor::WatchlistInteractorImpl::new(
            db_pool,
            price_service.clone(),
            token_repository,
        ),
    );
    let view = Arc::new(crate::view::watchlist_view::TelegramWatchlistView::new(
        bot.clone(),
        chat_id,
    ));
    let presenter = crate::presenter::watchlist_presenter::WatchlistPresenterImpl::new(
        interactor,
        view,
        price_service,
    );

    // Refresh watchlist
    presenter.refresh_watchlist(telegram_id).await?;

    Ok(())
}

// Function to view token details
async fn handle_watchlist_view_token(
    bot: &Bot,
    token_address: &str,
    message: Message,
    telegram_id: i64,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let chat_id = message.chat.id;

    // Create presenter
    let db_pool = services.db_pool();
    let price_service = services.price_service();
    let token_repository = services.token_repository();

    let interactor = Arc::new(
        crate::interactor::watchlist_interactor::WatchlistInteractorImpl::new(
            db_pool,
            price_service.clone(),
            token_repository,
        ),
    );
    let view = Arc::new(crate::view::watchlist_view::TelegramWatchlistView::new(
        bot.clone(),
        chat_id,
    ));
    let presenter = crate::presenter::watchlist_presenter::WatchlistPresenterImpl::new(
        interactor,
        view,
        price_service,
    );

    // Show token details
    presenter
        .show_token_detail(telegram_id, token_address)
        .await?;

    Ok(())
}

// Function to remove token from watchlist
async fn handle_watchlist_remove_token(
    bot: &Bot,
    token_address: &str,
    message: Message,
    telegram_id: i64,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let chat_id = message.chat.id;

    // Create presenter
    let db_pool = services.db_pool();
    let price_service = services.price_service();
    let token_repository = services.token_repository();

    let interactor = Arc::new(
        crate::interactor::watchlist_interactor::WatchlistInteractorImpl::new(
            db_pool,
            price_service.clone(),
            token_repository,
        ),
    );
    let view = Arc::new(crate::view::watchlist_view::TelegramWatchlistView::new(
        bot.clone(),
        chat_id,
    ));
    let presenter = crate::presenter::watchlist_presenter::WatchlistPresenterImpl::new(
        interactor,
        view,
        price_service,
    );

    // Remove token from watchlist
    presenter
        .remove_from_watchlist(telegram_id, token_address)
        .await?;

    Ok(())
}

// Function to start the withdraw flow
async fn handle_withdraw_start(
    bot: &Bot,
    message: Message,
    telegram_id: i64,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let chat_id = message.chat.id;

    // Update dialogue state
    dialogue
        .update(State::AwaitingWithdrawTokenSelection)
        .await?;

    // Create presenter
    let db_pool = services.db_pool();
    let solana_client = services.solana_client();
    let price_service = services.price_service();

    let interactor = Arc::new(
        crate::interactor::withdraw_interactor::WithdrawInteractorImpl::new(
            db_pool,
            solana_client,
            price_service,
        ),
    );
    let view = Arc::new(crate::view::withdraw_view::TelegramWithdrawView::new(
        bot.clone(),
        chat_id,
    ));
    let presenter =
        crate::presenter::withdraw_presenter::WithdrawPresenterImpl::new(interactor, view);

    // Start the withdraw flow
    presenter.start_withdraw_flow(telegram_id).await?;

    Ok(())
}

// Function to handle token selection
async fn handle_withdraw_token_selection(
    bot: &Bot,
    token_address: &str,
    message: Message,
    telegram_id: i64,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let chat_id = message.chat.id;

    // Create presenter and interactor
    let db_pool = services.db_pool();
    let solana_client = services.solana_client();
    let price_service = services.price_service();

    let interactor = Arc::new(
        crate::interactor::withdraw_interactor::WithdrawInteractorImpl::new(
            db_pool.clone(),
            solana_client.clone(),
            price_service.clone(),
        ),
    );

    // Get token info and balance
    match interactor.get_user_tokens(telegram_id).await {
        Ok(tokens) => {
            let token = tokens.iter().find(|t| t.mint_address == token_address);

            if let Some(token_balance) = token {
                // Get current token price
                match interactor.get_token_price(token_address).await {
                    Ok((price_in_sol, price_in_usdc)) => {
                        // Update dialogue state
                        dialogue
                            .update(State::AwaitingWithdrawRecipientAddress {
                                token_address: token_address.to_string(),
                                token_symbol: token_balance.symbol.clone(),
                                amount: token_balance.amount,
                                price_in_sol,
                                price_in_usdc,
                            })
                            .await?;

                        // Calculate total values
                        let total_sol_value = token_balance.amount * price_in_sol;
                        let total_usdc_value = token_balance.amount * price_in_usdc;

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

                        // Show token details and prompt for recipient
                        bot.send_message(
                            chat_id,
                            format!(
                                "<b>{} Token Details</b>\n\n\
                                • Symbol: <b>{}</b>\n\
                                • Address: <code>{}</code>\n\
                                • Your Balance: <b>{:.6}</b>\n\
                                • Price: <b>{:.6} SOL</b> (${:.2})\n\
                                • Total Value: <b>{:.6} SOL</b> (${:.2})\n\n\
                                Enter the recipient's Solana address:",
                                token_balance.symbol,
                                token_balance.symbol,
                                short_address,
                                token_balance.amount,
                                price_in_sol,
                                price_in_usdc,
                                total_sol_value,
                                total_usdc_value
                            ),
                        )
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                    }
                    Err(e) => {
                        bot.send_message(chat_id, format!("Error getting token price: {}", e))
                            .await?;
                    }
                }
            } else {
                bot.send_message(
                    chat_id,
                    format!(
                        "Token with address {} not found in your wallet",
                        token_address
                    ),
                )
                .await?;
            }
        }
        Err(e) => {
            bot.send_message(chat_id, format!("Error retrieving tokens: {}", e))
                .await?;
        }
    }

    Ok(())
}

// Function to start the sell flow with token selection
async fn handle_sell_start(
    bot: &Bot,
    message: Message,
    telegram_id: i64,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let chat_id = message.chat.id;

    // Update dialogue state
    dialogue.update(State::AwaitingSellTokenSelection).await?;

    // Get user's tokens
    let db_pool = services.db_pool();
    let solana_client = services.solana_client();

    match crate::commands::trade::get_user_tokens(
        telegram_id,
        db_pool.clone(),
        solana_client.clone(),
    )
    .await
    {
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

// Function to handle token selection for sell
async fn handle_sell_token_selection(
    bot: &Bot,
    token_address: &str,
    message: Message,
    telegram_id: i64,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let chat_id = message.chat.id;

    // Get token info and current price
    let db_pool = services.db_pool();
    let solana_client = services.solana_client();
    let price_service = services.price_service();

    // Get user's tokens
    match crate::commands::trade::get_user_tokens(
        telegram_id,
        db_pool.clone(),
        solana_client.clone(),
    )
    .await
    {
        Ok(tokens) => {
            // Find the selected token
            if let Some(token) = tokens.iter().find(|t| t.mint_address == token_address) {
                // Get token price
                match price_service.get_token_price(token_address).await {
                    Ok(price_info) => {
                        let price_in_sol = price_info.price_in_sol;
                        let price_in_usdc = price_info.price_in_usdc;

                        // Calculate total value
                        let total_value_sol = token.amount * price_in_sol;
                        let total_value_usdc = token.amount * price_in_usdc;

                        // Update dialogue state
                        dialogue
                            .update(State::AwaitingSellAmount {
                                token_address: token_address.to_string(),
                                token_symbol: token.symbol.clone(),
                                balance: token.amount,
                                price_in_sol,
                                price_in_usdc,
                            })
                            .await?;

                        // Display token details and prompt for amount
                        bot.send_message(
                            chat_id,
                            format!(
                                "<b>{} Token Details</b>\n\n\
                                • Symbol: <b>{}</b>\n\
                                • Your Balance: <b>{:.6}</b>\n\
                                • Current Price: <b>{:.6} SOL</b> (${:.2})\n\
                                • Total Value: <b>{:.6} SOL</b> (${:.2})\n\n\
                                How many tokens do you want to sell?\n\
                                • Enter a specific amount (e.g. <code>10.5</code>)\n\
                                • Enter a percentage (e.g. <code>50%</code>)\n\
                                • Or type <code>All</code> to sell your entire balance",
                                token.symbol,
                                token.symbol,
                                token.amount,
                                price_in_sol,
                                price_in_usdc,
                                total_value_sol,
                                total_value_usdc
                            ),
                        )
                        .parse_mode(ParseMode::Html)
                        .await?;
                    }
                    Err(e) => {
                        bot.send_message(chat_id, format!("Error getting token price: {}", e))
                            .await?;
                    }
                }
            } else {
                bot.send_message(
                    chat_id,
                    format!(
                        "Token with address {} not found in your wallet",
                        token_address
                    ),
                )
                .await?;
            }
        }
        Err(e) => {
            bot.send_message(chat_id, format!("Error retrieving tokens: {}", e))
                .await?;
        }
    }

    Ok(())
}

// Function to start the buy flow with token selection
async fn handle_buy_start(
    bot: &Bot,
    message: Message,
    telegram_id: i64,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let chat_id = message.chat.id;

    // Update dialogue state
    dialogue.update(State::AwaitingBuyTokenSelection).await?;

    // Create set to track token addresses to avoid duplicates
    let mut token_addresses = std::collections::HashSet::new();
    let mut keyboard_buttons = Vec::new();

    // Step 1: Get user's existing tokens
    let db_pool = services.db_pool();
    let solana_client = services.solana_client();

    if let Ok(user_tokens) =
        crate::commands::trade::get_user_tokens(telegram_id, db_pool.clone(), solana_client.clone())
            .await
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

// Function to handle manual address entry
async fn handle_buy_manual_address(
    bot: &Bot,
    message: Message,
    dialogue: MyDialogue,
) -> Result<()> {
    let chat_id = message.chat.id;

    // Update dialogue state
    dialogue.update(State::AwaitingBuyManualAddress).await?;

    // Prompt for address
    bot.send_message(chat_id, "Please enter the token contract address:")
        .await?;

    Ok(())
}

// Function to handle token selection
async fn handle_buy_token_selection(
    bot: &Bot,
    token_address: &str,
    message: Message,
    telegram_id: i64,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let chat_id = message.chat.id;

    // Create services for token info
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

    // Get token information
    match interactor.get_token_info(token_address).await {
        Ok((token_symbol, price_in_sol, price_in_usdc)) => {
            // Update dialogue state
            dialogue
                .update(State::AwaitingBuyAmount {
                    token_address: token_address.to_string(),
                    token_symbol: token_symbol.clone(),
                    price_in_sol,
                    price_in_usdc,
                })
                .await?;

            // Display token info with pricing
            bot.send_message(
                chat_id,
                format!(
                    "<b>{} Token Details</b>\n\n\
                    • Symbol: <b>{}</b>\n\
                    • Address: <code>{}</code>\n\
                    • Current Price: <b>{:.6} SOL</b> (${:.2})\n\n\
                    How many tokens do you want to buy?",
                    token_symbol, token_symbol, token_address, price_in_sol, price_in_usdc
                ),
            )
            .parse_mode(ParseMode::Html)
            .await?;
        }
        Err(e) => {
            bot.send_message(chat_id, format!("Error getting token info: {}", e))
                .await?;
        }
    }

    Ok(())
}
