//! Solana Wallet Bot for Telegram
//!
//! This library provides the core functionality for a Telegram bot that allows users to
//! create and manage Solana wallets, check balances, perform token swaps via Jupiter,
//! and execute trades directly from Telegram chats.
//!
/// Command handlers for bot interactions
pub mod commands;
/// Dependency injection container
pub mod di;
/// Domain entities and data structures
pub mod entity;
/// Business logic interactors
pub mod interactor;
/// Presentation layer
pub mod presenter;
/// QR code utility functions
pub mod qrcodeutils;
/// Command routing
pub mod router;
/// Solana blockchain interactions
pub mod solana;
/// Utility functions
pub mod utils;
/// View layer for rendering responses
pub mod view;

pub mod services;

// Re-export commonly used items for convenient imports
pub use commands::BotCommands;
pub use di::ServiceContainer;
pub use entity::{State, TokenBalance, User};
pub use interactor::db;
pub use presenter::Presenter;
pub use router::{Router, TelegramRouter};
pub use solana::create_solana_client;
use teloxide::dispatching::dialogue::InMemStorage;
pub use utils::{generate_qr_code, validate_solana_address};

/// Version of the library
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Create and initialize the application with all dependencies
///
/// This function serves as the main entry point for creating a fully
/// configured application instance with all necessary dependencies.
///
/// # Arguments
///
/// * `bot` - Telegram bot instance
/// * `db_pool` - Database connection pool
/// * `solana_client` - Initialized Solana client
pub fn create_application(
    bot: teloxide::Bot,
    db_pool: std::sync::Arc<sqlx::PgPool>,
    solana_client: std::sync::Arc<solana_client::nonblocking::rpc_client::RpcClient>,
) -> (
    TelegramRouter,
    teloxide::Bot,
    std::sync::Arc<ServiceContainer>,
    std::sync::Arc<InMemStorage<State>>,
    services::LimitOrderService,
) {
    use std::sync::Arc;
    use teloxide::dispatching::dialogue::InMemStorage;

    // Create service container
    let service_container = Arc::new(ServiceContainer::new(db_pool, solana_client));

    // In-memory storage for dialogues
    let storage = InMemStorage::<State>::new();

    // Create the router
    let router = TelegramRouter::new(service_container.clone());

    // Create limit order service
    let limit_order_service =
        services::LimitOrderService::new(service_container.clone(), bot.clone());

    (router, bot, service_container, storage, limit_order_service)
}
