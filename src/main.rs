use tokio;
use dotenv::dotenv;
use env_logger;
use teloxide::{prelude::*, Bot};
use std::env;
use log::{info, error};
use sqlx::{PgPool, postgres::PgPoolOptions};

mod commands;
mod db;
mod solana;
mod raydium;
mod utils;
mod model;
mod qrcodeutils;

use teloxide::dispatching::dialogue::InMemStorage;

type MyDialogue = Dialogue<model::State, InMemStorage<model::State>>;

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

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in environment variables");
    let db_pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("Failed to create database connection pool");

    // Run migrations
    info!("Running database migrations...");
    match sqlx::migrate!("./migrations").run(&db_pool).await {
        Ok(_) => info!("Migrations completed successfully"),
        Err(e) => {
            error!("Failed to run migrations: {}", e);
            return Err(anyhow::Error::from(e));
        }
    }

    let solana_rpc_url = env::var("SOLANA_RPC_URL")
        .expect("SOLANA_RPC_URL must be set in environment variables");
    let solana_client = solana::create_solana_client(&solana_rpc_url)
        .expect("Failed to create Solana client");

    // In-memory storage (could replace with persistent storage if needed)
    let storage = InMemStorage::<model::State>::new();

    // Setup command handlers
    let handler = commands::setup_command_handlers();

    // Create dependency tree
    let dependencies = dptree::deps![db_pool, solana_client, storage];

    // Build dispatcher with control-C handling enabled
    let mut dispatcher = Dispatcher::builder(bot, handler)
        .dependencies(dependencies)
        .enable_ctrlc_handler()
        .build();

    info!("Bot is running!");
    dispatcher.dispatch().await; // Launch dispatcher

    Ok(())
}