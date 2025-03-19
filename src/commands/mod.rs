// src/commands/mod.rs
use anyhow::Result;
use sqlx::PgPool;
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use teloxide::{
    prelude::*,
    dispatching::dialogue::InMemStorage,
    types::ParseMode,
};

use crate::model::State;
use crate::MyDialogue;

pub mod start;
pub mod wallet;
pub mod balance;
pub mod send;
pub mod swap;
pub mod price;
pub mod help;

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
        dialogue: Option<MyDialogue>,
        db_pool: Option<PgPool>,
        solana_client: Option<Arc<RpcClient>>
    ) -> Result<()>;
}

/// Register all command handlers in the command system
pub fn register_commands() -> Vec<(&'static str, &'static str)> {
    vec![
        (start::StartCommand::command_name(), start::StartCommand::description()),
        (wallet::CreateWalletCommand::command_name(), wallet::CreateWalletCommand::description()),
        (wallet::AddressCommand::command_name(), wallet::AddressCommand::description()),
        (balance::BalanceCommand::command_name(), balance::BalanceCommand::description()),
        (send::SendCommand::command_name(), send::SendCommand::description()),
        (swap::SwapCommand::command_name(), swap::SwapCommand::description()),
        (price::PriceCommand::command_name(), price::PriceCommand::description()),
        (help::HelpCommand::command_name(), help::HelpCommand::description()),
    ]
}

/// Setup the command handlers for the bot
pub fn setup_command_handlers() -> teloxide::dispatching::UpdateHandler<anyhow::Error> {
    use teloxide::dispatching::UpdateFilterExt;
    use dptree::case;

    // Use BotCommands enum with teloxide's command filter
    let command_handler = teloxide::filter_command::<BotCommands, _>()
        .branch(case![BotCommands::Start].endpoint(|bot: Bot, msg: Message, db_pool: PgPool| async move {
            start::StartCommand::execute(bot, msg, None, Some(db_pool), None).await
        }))
        .branch(case![BotCommands::CreateWallet].endpoint(|bot: Bot, msg: Message, db_pool: PgPool| async move {
            wallet::CreateWalletCommand::execute(bot, msg, None, Some(db_pool), None).await
        }))
        .branch(case![BotCommands::Address].endpoint(|bot: Bot, msg: Message, db_pool: PgPool| async move {
            wallet::AddressCommand::execute(bot, msg, None, Some(db_pool), None).await
        }))
        .branch(case![BotCommands::Balance].endpoint(|bot: Bot, msg: Message, db_pool: PgPool, solana_client: Arc<RpcClient>| async move {
            balance::BalanceCommand::execute(bot, msg, None, Some(db_pool), Some(solana_client)).await
        }))
        .branch(case![BotCommands::Send].endpoint(|bot: Bot, msg: Message, dialogue: MyDialogue| async move {
            send::SendCommand::execute(bot, msg, Some(dialogue), None, None).await
        }))
        .branch(case![BotCommands::Swap].endpoint(|bot: Bot, msg: Message, db_pool: PgPool, solana_client: Arc<RpcClient>| async move {
            swap::SwapCommand::execute(bot, msg, None, Some(db_pool), Some(solana_client)).await
        }))
        .branch(case![BotCommands::Price].endpoint(|bot: Bot, msg: Message| async move {
            price::PriceCommand::execute(bot, msg, None, None, None).await
        }))
        .branch(case![BotCommands::Help].endpoint(|bot: Bot, msg: Message| async move {
            help::HelpCommand::execute(bot, msg, None, None, None).await
        }));

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(dptree::entry()
            .branch(case![State::AwaitingRecipientAddress].endpoint(send::receive_recipient_address))
            .branch(case![State::AwaitingAmount { recipient }].endpoint(send::receive_amount))
            .branch(case![State::AwaitingConfirmation { recipient, amount, token }].endpoint(send::receive_confirmation))
            .branch(case![State::AwaitingSwapDetails].endpoint(swap::receive_swap_details))
        );

    teloxide::dispatching::dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(message_handler)
}

/// Bot Commands enum for teloxide command filter
#[derive(teloxide::utils::command::BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum BotCommands {
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