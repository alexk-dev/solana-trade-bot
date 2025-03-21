use async_trait::async_trait;
use std::sync::Arc;
use teloxide::{
    dispatching::dialogue::Dialogue, dispatching::dialogue::InMemStorage,
    dispatching::UpdateHandler, prelude::*,
};

use crate::commands::{self, BotCommands, CommandHandler};
use crate::di::ServiceContainer;
use crate::entity::State;

type MyDialogue = Dialogue<State, InMemStorage<State>>;

// Base router trait
#[async_trait]
pub trait Router: Send + Sync {
    fn setup_handlers(&self) -> UpdateHandler<anyhow::Error>;
}

// Command router implementation
pub struct TelegramRouter {
    services: Arc<ServiceContainer>,
}

impl TelegramRouter {
    pub fn new(services: Arc<ServiceContainer>) -> Self {
        Self { services }
    }
}

#[async_trait]
impl Router for TelegramRouter {
    fn setup_handlers(&self) -> UpdateHandler<anyhow::Error> {
        use dptree::case;
        use teloxide::dispatching::UpdateFilterExt;

        let services1 = self.services.clone();
        let services2 = self.services.clone();
        let services3 = self.services.clone();
        let services4 = self.services.clone();
        let services5 = self.services.clone();
        let services6 = self.services.clone();
        let services7 = self.services.clone();
        let services8 = self.services.clone();

        // Use BotCommands enum with teloxide's command filter
        let command_handler = teloxide::filter_command::<BotCommands, _>()
            .branch(case![BotCommands::Start].endpoint(
                move |bot: Bot, msg: Message, _dialogue: MyDialogue| {
                    let services_local = services1.clone();
                    async move {
                        commands::start::StartCommand::execute(bot, msg, None, services_local).await
                    }
                },
            ))
            .branch(case![BotCommands::CreateWallet].endpoint(
                move |bot: Bot, msg: Message, dialogue: MyDialogue| {
                    let services_local = services2.clone();
                    async move {
                        commands::wallet::CreateWalletCommand::execute(
                            bot,
                            msg,
                            Some(dialogue),
                            services_local,
                        )
                        .await
                    }
                },
            ))
            .branch(case![BotCommands::Address].endpoint(
                move |bot: Bot, msg: Message, dialogue: MyDialogue| {
                    let services_local = services3.clone();
                    async move {
                        commands::wallet::AddressCommand::execute(
                            bot,
                            msg,
                            Some(dialogue),
                            services_local,
                        )
                        .await
                    }
                },
            ))
            .branch(case![BotCommands::Balance].endpoint(
                move |bot: Bot, msg: Message, dialogue: MyDialogue| {
                    let services_local = services4.clone();
                    async move {
                        commands::balance::BalanceCommand::execute(
                            bot,
                            msg,
                            Some(dialogue),
                            services_local,
                        )
                        .await
                    }
                },
            ))
            .branch(case![BotCommands::Send].endpoint(
                move |bot: Bot, msg: Message, dialogue: MyDialogue| {
                    let services_local = services5.clone();
                    async move {
                        commands::send::SendCommand::execute(
                            bot,
                            msg,
                            Some(dialogue),
                            services_local,
                        )
                        .await
                    }
                },
            ))
            .branch(case![BotCommands::Swap].endpoint(
                move |bot: Bot, msg: Message, dialogue: MyDialogue| {
                    let services_local = services6.clone();
                    async move {
                        commands::swap::SwapCommand::execute(
                            bot,
                            msg,
                            Some(dialogue),
                            services_local,
                        )
                        .await
                    }
                },
            ))
            .branch(case![BotCommands::Price].endpoint(
                move |bot: Bot, msg: Message, dialogue: MyDialogue| {
                    let services_local = services7.clone();
                    async move {
                        commands::price::PriceCommand::execute(
                            bot,
                            msg,
                            Some(dialogue),
                            services_local,
                        )
                        .await
                    }
                },
            ))
            .branch(case![BotCommands::Help].endpoint(
                move |bot: Bot, msg: Message, dialogue: MyDialogue| {
                    let services_local = services8.clone();
                    async move {
                        commands::help::HelpCommand::execute(
                            bot,
                            msg,
                            Some(dialogue),
                            services_local,
                        )
                        .await
                    }
                },
            ));

        let services_for_dialog1 = self.services.clone();
        let services_for_dialog2 = self.services.clone();
        let services_for_dialog3 = self.services.clone();
        let services_for_dialog4 = self.services.clone();

        let message_handler =
            Update::filter_message().branch(command_handler).branch(
                dptree::entry()
                    .branch(case![State::AwaitingRecipientAddress].endpoint(
                        move |bot: Bot, msg: Message, dialogue: MyDialogue| {
                            let services = services_for_dialog1.clone();
                            async move {
                                commands::send::receive_recipient_address(
                                    bot, msg, dialogue, services,
                                )
                                .await
                            }
                        },
                    ))
                    .branch(case![State::AwaitingAmount { recipient }].endpoint(
                        move |bot: Bot, msg: Message, state: State, dialogue: MyDialogue| {
                            let services = services_for_dialog2.clone();
                            async move {
                                commands::send::receive_amount(bot, msg, state, dialogue, services)
                                    .await
                            }
                        },
                    ))
                    .branch(
                        case![State::AwaitingConfirmation {
                            recipient,
                            amount,
                            token
                        }]
                        .endpoint(
                            move |bot: Bot, msg: Message, state: State, dialogue: MyDialogue| {
                                let services = services_for_dialog3.clone();
                                async move {
                                    commands::send::receive_confirmation(
                                        bot, msg, state, dialogue, services,
                                    )
                                    .await
                                }
                            },
                        ),
                    )
                    .branch(
                        case![State::AwaitingSwapDetails].endpoint(
                            move |bot: Bot, msg: Message, dialogue: MyDialogue| {
                                let services = services_for_dialog4.clone();
                                async move {
                                    commands::swap::receive_swap_details(bot, msg, dialogue).await
                                }
                            },
                        ),
                    ),
            );

        teloxide::dispatching::dialogue::enter::<Update, InMemStorage<State>, State, _>()
            .branch(message_handler)
    }
}
