use anyhow::Result;
use std::sync::Arc;
use teloxide::{dispatching::dialogue::InMemStorage, prelude::*};

use crate::di::ServiceContainer; // Import the service container
use crate::entity::State;
use teloxide::dispatching::dialogue::Dialogue;
type MyDialogue = Dialogue<State, InMemStorage<State>>;

pub mod balance;
pub mod help;
pub mod price;
pub mod send;
pub mod start;
pub mod swap;
pub mod wallet;

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
            balance::BalanceCommand::command_name(),
            balance::BalanceCommand::description(),
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

/// Setup the command handlers for the bot
pub fn setup_command_handlers() -> teloxide::dispatching::UpdateHandler<anyhow::Error> {
    use dptree::case;
    use teloxide::dispatching::UpdateFilterExt;

    // Use BotCommands enum with teloxide's command filter
    let command_handler =
        teloxide::filter_command::<BotCommands, _>()
            .branch(
                case![BotCommands::Start].endpoint(
                    |bot: Bot,
                     msg: Message,
                     dialogue: MyDialogue,
                     services: Arc<ServiceContainer>| async move {
                        start::StartCommand::execute(bot, msg, Some(dialogue), services).await
                    },
                ),
            )
            .branch(
                case![BotCommands::CreateWallet].endpoint(
                    |bot: Bot,
                     msg: Message,
                     dialogue: MyDialogue,
                     services: Arc<ServiceContainer>| async move {
                        wallet::CreateWalletCommand::execute(bot, msg, Some(dialogue), services)
                            .await
                    },
                ),
            )
            .branch(
                case![BotCommands::Address].endpoint(
                    |bot: Bot,
                     msg: Message,
                     dialogue: MyDialogue,
                     services: Arc<ServiceContainer>| async move {
                        wallet::AddressCommand::execute(bot, msg, Some(dialogue), services).await
                    },
                ),
            )
            .branch(
                case![BotCommands::Balance].endpoint(
                    |bot: Bot,
                     msg: Message,
                     dialogue: MyDialogue,
                     services: Arc<ServiceContainer>| async move {
                        balance::BalanceCommand::execute(bot, msg, Some(dialogue), services).await
                    },
                ),
            )
            .branch(
                case![BotCommands::Send].endpoint(
                    |bot: Bot,
                     msg: Message,
                     dialogue: MyDialogue,
                     services: Arc<ServiceContainer>| async move {
                        send::SendCommand::execute(bot, msg, Some(dialogue), services).await
                    },
                ),
            )
            .branch(
                case![BotCommands::Swap].endpoint(
                    |bot: Bot,
                     msg: Message,
                     dialogue: MyDialogue,
                     services: Arc<ServiceContainer>| async move {
                        swap::SwapCommand::execute(bot, msg, Some(dialogue), services).await
                    },
                ),
            )
            .branch(
                case![BotCommands::Price].endpoint(
                    |bot: Bot,
                     msg: Message,
                     dialogue: MyDialogue,
                     services: Arc<ServiceContainer>| async move {
                        price::PriceCommand::execute(bot, msg, Some(dialogue), services).await
                    },
                ),
            )
            .branch(
                case![BotCommands::Help].endpoint(
                    |bot: Bot,
                     msg: Message,
                     dialogue: MyDialogue,
                     services: Arc<ServiceContainer>| async move {
                        help::HelpCommand::execute(bot, msg, Some(dialogue), services).await
                    },
                ),
            );

    let message_handler = Update::filter_message().branch(command_handler).branch(
        dptree::entry()
            .branch(
                case![State::AwaitingRecipientAddress].endpoint(send::receive_recipient_address),
            )
            .branch(case![State::AwaitingAmount { recipient }].endpoint(send::receive_amount))
            .branch(
                case![State::AwaitingConfirmation {
                    recipient,
                    amount,
                    token
                }]
                .endpoint(
                    |bot: Bot,
                     msg: Message,
                     state: State,
                     dialogue: MyDialogue,
                     services: Arc<ServiceContainer>| async move {
                        send::receive_confirmation(bot, msg, state, dialogue, services).await
                    },
                ),
            )
            .branch(case![State::AwaitingSwapDetails].endpoint(
                |bot: Bot, msg: Message, dialogue: MyDialogue| async move {
                    swap::receive_swap_details(bot, msg, dialogue).await
                },
            )),
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
    #[command(
        description = "swap tokens via Raydium (format: /swap amount from_token to_token slippage%)"
    )]
    Swap,
    #[command(description = "get price for a token")]
    Price,
    #[command(description = "display this help message")]
    Help,
}
