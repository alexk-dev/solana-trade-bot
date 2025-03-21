use dotenv::dotenv;
use env_logger;
use log::{error, info};
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::sync::Arc;
use teloxide::{prelude::*, Bot};
use tokio;

mod commands;
mod di;
mod entity;
mod interactor;
mod model;
mod presenter;
mod qrcodeutils;
mod router;
mod solana;
mod utils;
mod view;

use di::ServiceContainer;
use router::{Router, TelegramRouter};
use teloxide::dispatching::dialogue::InMemStorage;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    // Initialize logging
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    info!("Starting Solana Wallet Telegram Bot");

    // Load environment variables
    let bot_token = env::var("TELEGRAM_BOT_TOKEN")
        .expect("TELEGRAM_BOT_TOKEN must be set in environment variables");
    let bot = Bot::new(bot_token);

    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL must be set in environment variables");
    let db_pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("Failed to create database connection pool");
    let db_pool = std::sync::Arc::new(db_pool);

    let db_pool_for_migration = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("Failed to create database connection pool");

    // Run migrations
    info!("Running database migrations...");
    match sqlx::migrate!("./migrations")
        .run(&db_pool_for_migration)
        .await
    {
        Ok(_) => info!("Migrations completed successfully"),
        Err(e) => {
            error!("Failed to run migrations: {}", e);
            return Err(anyhow::Error::from(e));
        }
    }

    db_pool_for_migration.close().await;

    let solana_rpc_url =
        env::var("SOLANA_RPC_URL").expect("SOLANA_RPC_URL must be set in environment variables");
    let solana_client =
        solana::create_solana_client(&solana_rpc_url).expect("Failed to create Solana client");

    // Create service container
    let service_container = ServiceContainer::new(db_pool, solana_client.clone());
    let service_container = Arc::new(service_container);

    // In-memory storage for dialogues
    let storage = InMemStorage::<entity::State>::new();

    // Create and setup the router
    let router = TelegramRouter::new(service_container.clone());
    let handler = router.setup_handlers();

    // Build dispatcher with dependency injections and control-C handling
    let mut dispatcher = Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![service_container, storage])
        .enable_ctrlc_handler()
        .build();

    info!("Bot is running!");
    dispatcher.dispatch().await;

    Ok(())
}
