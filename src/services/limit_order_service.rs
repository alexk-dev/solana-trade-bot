use crate::di::ServiceContainer;
use crate::entity::{LimitOrder, LimitOrderStatus, OrderType, WatchlistItem};
use crate::interactor::db;
use crate::interactor::trade_interactor::{TradeInteractor, TradeInteractorImpl};
use crate::solana::jupiter::price_service::PriceService;
use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use teloxide::{prelude::*, types::ParseMode, Bot};
use tokio::select;
use tokio::sync::mpsc;
use tokio::time::{interval, sleep, Instant};

pub struct LimitOrderService {
    services: Arc<ServiceContainer>,
    bot: Bot,
    stop_tx: Option<mpsc::Sender<()>>,
}

impl LimitOrderService {
    pub fn new(services: Arc<ServiceContainer>, bot: Bot) -> Self {
        Self {
            services,
            bot,
            stop_tx: None,
        }
    }

    // Start the background service that monitors and executes limit orders
    pub async fn start(&mut self) -> Result<()> {
        if self.stop_tx.is_some() {
            warn!("Limit order service is already running");
            return Ok(());
        }

        // Create a channel for stopping the service
        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
        self.stop_tx = Some(stop_tx);

        let services_clone = self.services.clone();
        let bot_clone = self.bot.clone();

        // Spawn a new async task that runs independently
        tokio::spawn(async move {
            // Create an interval ticker that triggers every 13 seconds
            let mut interval = interval(Duration::from_secs(13));
            let mut last_run = Instant::now();

            loop {
                select! {
                    // When the interval ticks, process limit orders
                    _ = interval.tick() => {
                        let elapsed = last_run.elapsed();
                        debug!("Running limit order check (last run: {:.2?} ago)", elapsed);

                        if let Err(e) = Self::process_limit_orders_and_watchlist(&services_clone, &bot_clone).await {
                            error!("Error processing limit orders and watchlist: {}", e);
                        }

                        last_run = Instant::now();
                    }
                    // When we receive a stop signal, exit the loop
                    _ = stop_rx.recv() => {
                        info!("Stopping limit order service");
                        break;
                    }
                }
            }
        });

        info!("Limit order service started");
        Ok(())
    }

    // Stop the background service
    pub async fn stop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(()).await;
            info!("Limit order service stop signal sent");
        }
    }

    // Enhanced process function that handles both limit orders and watchlist
    async fn process_limit_orders_and_watchlist(services: &Arc<ServiceContainer>, bot: &Bot) -> Result<()> {
        let db_pool = services.db_pool();

        // Collect all the token addresses we need to check prices for
        let mut all_tokens = HashMap::new();
        let mut token_prices = HashMap::new();

        // 1. Get all active limit orders
        let active_orders = db::get_all_active_limit_orders(&db_pool).await?;

        if !active_orders.is_empty() {
            info!("Processing {} active limit orders", active_orders.len());

            // Extract unique token addresses from orders
            for order in &active_orders {
                all_tokens.insert(
                    order.token_address.clone(),
                    order.token_symbol.clone()
                );
            }
        }

        // 2. Get all watchlist items from all users
        let mut all_users = HashSet::new();
        let mut watchlist_by_user = HashMap::new();

        // First, get all user IDs with non-empty watchlists
        let users = sqlx::query!("SELECT DISTINCT user_id FROM watchlist")
            .fetch_all(&*db_pool)
            .await?;

        for user_row in users {
            all_users.insert(user_row.user_id);
        }

        // Then get watchlist items for each user
        for user_id in &all_users {
            // Get user's telegram_id
            let user = db::get_user_by_id(&db_pool, *user_id).await?;
            let telegram_id = user.telegram_id;

            // Get user's watchlist
            let watchlist = db::get_user_watchlist(&db_pool, telegram_id).await?;

            // Add tokens to the collection
            for item in &watchlist {
                all_tokens.insert(
                    item.token_address.clone(),
                    item.token_symbol.clone()
                );
            }

            // Store watchlist for later updates
            if !watchlist.is_empty() {
                watchlist_by_user.insert(telegram_id, watchlist);
            }
        }

        // 3. Process all token prices in a single pass
        if !all_tokens.is_empty() {
            info!("Getting prices for {} unique tokens", all_tokens.len());

            let price_service = services.price_service();

            // Get price for each token (no duplicates)
            for (token_address, token_symbol) in all_tokens {
                match price_service.get_token_price(&token_address).await {
                    Ok(price_info) => {
                        let price_in_sol = price_info.price_in_sol;
                        debug!("Got price for {}: {} SOL", token_symbol, price_in_sol);

                        // Store price for later use
                        token_prices.insert(token_address.clone(), price_in_sol);

                        // 4. Update limit orders with this token
                        for order in active_orders.iter().filter(|o| o.token_address == token_address) {
                            if let Err(e) = db::update_limit_order_current_price(
                                &db_pool,
                                order.id,
                                price_in_sol
                            ).await {
                                error!("Failed to update limit order #{} price: {}", order.id, e);
                            }

                            // Check if we need to execute the order
                            let should_execute = match order.order_type.as_str() {
                                "BUY" => price_in_sol <= order.price_in_sol,
                                "SELL" => price_in_sol >= order.price_in_sol,
                                _ => false,
                            };

                            if should_execute {
                                info!(
                                    "Executing {} order #{} for {} {} at {} SOL (current price: {})",
                                    order.order_type,
                                    order.id,
                                    order.amount,
                                    order.token_symbol,
                                    order.price_in_sol,
                                    price_in_sol
                                );

                                if let Err(e) = Self::execute_order(services, bot, order, price_in_sol).await {
                                    error!("Failed to execute order #{}: {}", order.id, e);
                                }
                            }
                        }

                        // 5. Update watchlist items with this token
                        for (telegram_id, watchlist) in &watchlist_by_user {
                            for item in watchlist.iter().filter(|w| w.token_address == token_address) {
                                if let Err(e) = db::update_watchlist_price(
                                    &db_pool,
                                    *telegram_id,
                                    &token_address,
                                    price_in_sol
                                ).await {
                                    error!("Failed to update watchlist price for user {}, token {}: {}", 
                                        telegram_id, token_symbol, e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to get price for token {}: {}", token_symbol, e);
                    }
                }

                // Add a small delay between API calls to avoid rate limiting
                sleep(Duration::from_millis(100)).await;
            }
        } else {
            debug!("No tokens to process");
        }

        Ok(())
    }

    // Execute a single limit order
    async fn execute_order(
        services: &Arc<ServiceContainer>,
        bot: &Bot,
        order: &LimitOrder,
        current_price: f64,
    ) -> Result<()> {
        let db_pool = services.db_pool();

        // Get user's telegram ID
        let user = db::get_user_by_id(&db_pool, order.user_id).await?;
        let telegram_id = user.telegram_id;

        let order_type = match order.order_type.as_str() {
            "BUY" => OrderType::Buy,
            "SELL" => OrderType::Sell,
            _ => return Err(anyhow!("Unknown order type: {}", order.order_type)),
        };

        // Create trade interactor
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

        // Execute the trade
        let result = interactor
            .execute_trade(
                telegram_id,
                &OrderType::from_str(&order.order_type).unwrap(),
                &order.token_address,
                &order.token_symbol,
                order.amount,
                current_price, // Use current market price
            )
            .await?;

        // Update order status based on trade result
        if result.success {
            // Mark order as filled
            db::update_limit_order_status(
                &db_pool,
                order.id,
                &LimitOrderStatus::Filled,
                result.signature.as_deref(),
            )
                .await?;

            // Notify user about successful execution
            bot.send_message(
                ChatId(telegram_id),
                format!(
                    "✅ <b>Limit Order Executed</b>\n\n\
                     Your limit {} order #{} has been filled:\n\
                     • {:.6} SOL ({:.6} {} tokens) at {:.6} SOL\n\
                     • Market price: {:.6} SOL\n\
                     • Transaction: <a href=\"https://explorer.solana.com/tx/{}\">View on Explorer</a>",
                    order.order_type,
                    order.id,
                    order.total_sol,
                    order.amount,
                    order.token_symbol,
                    order.price_in_sol,
                    current_price,
                    result.signature.unwrap_or_else(|| "unknown".to_string()),
                ),
            )
                .parse_mode(ParseMode::Html)
                .await?;
        } else {
            // Check retry count and potentially retry
            if order.retry_count < 2 {
                // Allow up to 3 attempts total (initial + 2 retries)
                // Increment retry count
                let new_retry_count = order.retry_count + 1;

                db::update_limit_order_retry_count(&db_pool, order.id, new_retry_count).await?;

                // Notify user about retry
                bot.send_message(
                    ChatId(telegram_id),
                    format!(
                        "⚠️ <b>Limit Order Retry</b>\n\n\
                         Your limit {} order #{} execution failed but will be retried automatically:\n\
                         • {:.6} SOL ({:.6} {} tokens) at {:.6} SOL\n\
                         • Market price: {:.6} SOL\n\
                         • Retry attempt: {} of 3\n\
                         • Error: {}",
                        order.order_type,
                        order.id,
                        order.total_sol,
                        order.amount,
                        order.token_symbol,
                        order.price_in_sol,
                        current_price,
                        new_retry_count,
                        result.error_message.unwrap_or_else(|| "Unknown error".to_string()),
                    ),
                )
                    .parse_mode(ParseMode::Html)
                    .await?;

                // Note: We don't mark it as failed, so it will be tried again next cycle
            } else {
                // We've exceeded retry attempts, mark as failed
                db::update_limit_order_status(&db_pool, order.id, &LimitOrderStatus::Failed, None)
                    .await?;

                // Notify user about failed execution after all retries
                bot.send_message(
                    ChatId(telegram_id),
                    format!(
                        "❌ <b>Limit Order Failed</b>\n\n\
                         Your limit {} order #{} could not be executed after 3 attempts:\n\
                         • {:.6} SOL ({:.6} {} tokens) at {:.6} SOL\n\
                         • Market price: {:.6} SOL\n\
                         • Error: {}\n\n\
                         The order has been marked as failed. Please check your wallet and try again.",
                        order.order_type,
                        order.id,
                        order.total_sol,
                        order.amount,
                        order.token_symbol,
                        order.price_in_sol,
                        current_price,
                        result.error_message.unwrap_or_else(|| "Unknown error".to_string()),
                    ),
                )
                    .parse_mode(ParseMode::Html)
                    .await?;
            }
        }

        Ok(())
    }
}