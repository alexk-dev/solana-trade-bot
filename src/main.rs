//! Solana Wallet Bot for Telegram - Main executable
//!
//! This is the entry point for the Telegram bot application that allows users
//! to create and manage Solana wallets, check balances, perform token swaps,
//! and execute trades directly from Telegram chats.
use anyhow::Context;
use dotenv::dotenv;
use log::{error, info};
use solana_trade_bot::{create_solana_client, Router};
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::sync::Arc;
use teloxide::{dptree, Bot};
use tokio;

/// Application entry point
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from .env file
    dotenv().ok();

    // Initialize logging with default level of "info"
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    info!(
        "Starting Solana Wallet Telegram Bot v{}",
        solana_trade_bot::VERSION
    );

    // Load and validate environment variables
    let bot_token = env::var("TELEGRAM_BOT_TOKEN")
        .context("TELEGRAM_BOT_TOKEN must be set in environment variables")?;

    let database_url =
        env::var("DATABASE_URL").context("DATABASE_URL must be set in environment variables")?;

    let solana_rpc_url = env::var("SOLANA_RPC_URL")
        .context("SOLANA_RPC_URL must be set in environment variables")?;

    // Create Telegram bot instance
    let bot = Bot::new(bot_token);

    // Setup database connection pool
    info!("Connecting to database...");
    let db_pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .context("Failed to create database connection pool")?;
    let db_pool = Arc::new(db_pool);

    // Create a separate connection for migrations
    let db_pool_for_migration = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .context("Failed to create migration connection pool")?;

    // Run database migrations
    info!("Running database migrations...");
    if let Err(e) = sqlx::migrate!("./migrations")
        .run(&db_pool_for_migration)
        .await
    {
        error!("Failed to run migrations: {}", e);
        return Err(anyhow::Error::from(e));
    }
    info!("Migrations completed successfully");

    // Close migration connection
    db_pool_for_migration.close().await;

    // Initialize Solana client
    info!("Connecting to Solana network...");
    let solana_client =
        create_solana_client(&solana_rpc_url).context("Failed to create Solana client")?;

    // Create and start the application
    info!("Initializing bot application...");

    // Initialize the application components
    let (router, bot, service_container, storage, mut limit_order_service) =
        solana_trade_bot::create_application(bot, db_pool, solana_client);

    // Start limit order background service
    info!("Starting limit order background service...");
    if let Err(e) = limit_order_service.start().await {
        error!("Failed to start limit order service: {}", e);
    } else {
        info!("Limit order service started successfully");
    }

    // Get the handler from the router
    let handler = router.setup_handlers();

    // Build dispatcher with dependency injections and control-C handling
    let mut dispatcher = teloxide::dispatching::Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![service_container, storage])
        .enable_ctrlc_handler()
        .build();

    info!("Bot is running! Press Ctrl+C to stop.");
    dispatcher.dispatch().await;

    // Stop limit order service
    info!("Stopping limit order service...");
    limit_order_service.stop().await;

    Ok(())
}
