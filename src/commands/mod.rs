use anyhow::Result;
use std::sync::Arc;
use teloxide::{
    dispatching::dialogue::InMemStorage,
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
};

use crate::di::ServiceContainer;
use crate::entity::State;
use teloxide::dispatching::dialogue::Dialogue;

pub mod callback;
pub mod help;
pub mod price;
pub mod send;
pub mod start;
pub mod swap;
pub mod ui;
pub mod wallet;

type MyDialogue = Dialogue<State, InMemStorage<State>>;

/// Trait that defines a command handler
pub trait CommandHandler {
    /// The command name in lowercase
    fn command_name() -> &'static str;

    /// The command description for help
    fn description() -> &'static str;

    /// Execute the command
    async fn execute(
        bot: Bot,
        msg: Message,
        telegram_id: i64,
        dialogue: Option<MyDialogue>,
        services: Arc<ServiceContainer>,
    ) -> Result<()>;
}

/// Register all command handlers in the command system
pub fn register_commands() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            start::StartCommand::command_name(),
            start::StartCommand::description(),
        ),
        (
            wallet::CreateWalletCommand::command_name(),
            wallet::CreateWalletCommand::description(),
        ),
        (
            wallet::AddressCommand::command_name(),
            wallet::AddressCommand::description(),
        ),
        (
            send::SendCommand::command_name(),
            send::SendCommand::description(),
        ),
        (
            swap::SwapCommand::command_name(),
            swap::SwapCommand::description(),
        ),
        (
            price::PriceCommand::command_name(),
            price::PriceCommand::description(),
        ),
        (
            help::HelpCommand::command_name(),
            help::HelpCommand::description(),
        ),
    ]
}

/// Bot Commands enum for teloxide command filter
#[derive(teloxide::utils::command::BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum BotCommands {
    #[command(description = "start the bot and show the main menu")]
    Start,
    #[command(rename = "create_wallet", description = "create a new Solana wallet")]
    CreateWallet,
    #[command(description = "show your wallet address and QR code")]
    Balance,
    #[command(description = "send funds to another address")]
    Send,
    #[command(description = "swap tokens via Jupiter")]
    Swap,
    #[command(description = "get price for a token")]
    Price,
    #[command(description = "display this help message")]
    Help,
    #[command(description = "show the main menu")]
    Menu,
}
