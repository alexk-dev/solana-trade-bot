use async_trait::async_trait;
use std::sync::Arc;
use teloxide::{
    dispatching::dialogue::Dialogue, dispatching::dialogue::InMemStorage,
    dispatching::UpdateHandler, prelude::*,
};

use crate::commands::{
    self, callback::handle_callback, trade, withdraw, BotCommands, CommandHandler,
};
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
        let services_for_callbacks = self.services.clone();

        // Use BotCommands enum with teloxide's command filter
        let command_handler = teloxide::filter_command::<BotCommands, _>()
            .branch(case![BotCommands::Start].endpoint(
                move |bot: Bot, msg: Message, _dialogue: MyDialogue| {
                    let services_local = services1.clone();
                    let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);
                    async move {
                        commands::start::StartCommand::execute(
                            bot,
                            msg,
                            telegram_id,
                            None,
                            services_local,
                        )
                        .await
                    }
                },
            ))
            .branch(case![BotCommands::Menu].endpoint(
                move |bot: Bot, msg: Message, dialogue: MyDialogue| {
                    let services_local = services2.clone();
                    let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);
                    async move {
                        commands::menu::MenuCommand::execute(
                            bot,
                            msg,
                            telegram_id,
                            Some(dialogue),
                            services_local,
                        )
                        .await
                    }
                },
            ))
            .branch(case![BotCommands::CreateWallet].endpoint(
                move |bot: Bot, msg: Message, dialogue: MyDialogue| {
                    let services_local = services3.clone();
                    let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);
                    async move {
                        commands::wallet::CreateWalletCommand::execute(
                            bot,
                            msg,
                            telegram_id,
                            Some(dialogue),
                            services_local,
                        )
                        .await
                    }
                },
            ))
            .branch(case![BotCommands::Help].endpoint(
                move |bot: Bot, msg: Message, dialogue: MyDialogue| {
                    let services_local = services4.clone();
                    let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);
                    async move {
                        commands::help::HelpCommand::execute(
                            bot,
                            msg,
                            telegram_id,
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
        let services_for_dialog5 = self.services.clone();
        let services_for_dialog6 = self.services.clone();
        let services_for_dialog7 = self.services.clone();
        let services_for_dialog8 = self.services.clone();
        let services_for_dialog9 = self.services.clone();
        let services_for_dialog10 = self.services.clone();
        let services_for_dialog11 = self.services.clone();
        let services_for_dialog12 = self.services.clone();
        let services_for_dialog13 = self.services.clone();
        let services_for_dialog14 = self.services.clone();
        let services_for_dialog15 = self.services.clone();
        let services_for_dialog16 = self.services.clone();
        let services_for_dialog17 = self.services.clone();
        let services_for_dialog18 = self.services.clone();

        let message_handler = Update::filter_message().branch(command_handler).branch(
            dptree::entry()
                .branch(
                    case![State::AwaitingWithdrawRecipientAddress {
                        token_address,
                        token_symbol,
                        amount,
                        price_in_sol,
                        price_in_usdc
                    }]
                    .endpoint(
                        move |bot: Bot, msg: Message, state: State, dialogue: MyDialogue| {
                            let services = services_for_dialog1.clone();
                            async move {
                                withdraw::receive_recipient_address(
                                    bot, msg, state, dialogue, services,
                                )
                                .await
                            }
                        },
                    ),
                )
                .branch(
                    case![State::AwaitingWithdrawAmount {
                        token_address,
                        token_symbol,
                        recipient,
                        balance,
                        price_in_sol,
                        price_in_usdc
                    }]
                    .endpoint(
                        move |bot: Bot, msg: Message, state: State, dialogue: MyDialogue| {
                            let services = services_for_dialog2.clone();
                            async move {
                                withdraw::receive_withdraw_amount(
                                    bot, msg, state, dialogue, services,
                                )
                                .await
                            }
                        },
                    ),
                )
                .branch(
                    case![State::AwaitingWithdrawConfirmation {
                        token_address,
                        token_symbol,
                        recipient,
                        amount,
                        price_in_sol,
                        total_sol,
                        total_usdc
                    }]
                    .endpoint(
                        move |bot: Bot, msg: Message, state: State, dialogue: MyDialogue| {
                            let services = services_for_dialog3.clone();
                            async move {
                                withdraw::receive_withdraw_confirmation(
                                    bot, msg, state, dialogue, services,
                                )
                                .await
                            }
                        },
                    ),
                )
                .branch(case![State::AwaitingPriceTokenAddress].endpoint(
                    move |bot: Bot, msg: Message, dialogue: MyDialogue| {
                        let services = services_for_dialog8.clone();
                        async move {
                            commands::price::receive_price_token_address(
                                bot, msg, dialogue, services,
                            )
                            .await
                        }
                    },
                ))
                .branch(
                    case![State::AwaitingLimitOrderTokenAddress { order_type }].endpoint(
                        move |bot: Bot, msg: Message, state: State, dialogue: MyDialogue| {
                            let services = services_for_dialog9.clone();
                            async move {
                                commands::limit_order::receive_token_address(
                                    bot, msg, state, dialogue, services,
                                )
                                .await
                            }
                        },
                    ),
                )
                .branch(
                    case![State::AwaitingLimitOrderPriceAndAmount {
                        order_type,
                        token_address,
                        token_symbol,
                        current_price_in_sol,
                        current_price_in_usdc
                    }]
                    .endpoint(
                        move |bot: Bot, msg: Message, state: State, dialogue: MyDialogue| {
                            let services = services_for_dialog10.clone();
                            async move {
                                commands::limit_order::receive_price_and_amount(
                                    bot, msg, state, dialogue, services,
                                )
                                .await
                            }
                        },
                    ),
                )
                .branch(
                    case![State::AwaitingLimitOrderConfirmation {
                        order_type,
                        token_address,
                        token_symbol,
                        price_in_sol,
                        amount,
                        total_sol
                    }]
                    .endpoint(
                        move |bot: Bot, msg: Message, state: State, dialogue: MyDialogue| {
                            let services = services_for_dialog11.clone();
                            async move {
                                commands::limit_order::receive_confirmation(
                                    bot, msg, state, dialogue, services,
                                )
                                .await
                            }
                        },
                    ),
                )
                .branch(case![State::AwaitingSlippageInput].endpoint(
                    move |bot: Bot, msg: Message, dialogue: MyDialogue| {
                        let services = services_for_dialog12.clone();
                        async move {
                            commands::settings::handle_slippage_input(bot, msg, dialogue, services)
                                .await
                        }
                    },
                ))
                .branch(case![State::AwaitingWatchlistTokenAddress].endpoint(
                    move |bot: Bot, msg: Message, dialogue: MyDialogue| {
                        let services = services_for_dialog13.clone();
                        async move {
                            commands::watchlist::handle_watchlist_token_address(
                                bot, msg, dialogue, services,
                            )
                            .await
                        }
                    },
                ))
                .branch(
                    case![State::AwaitingSellAmount {
                        token_address,
                        token_symbol,
                        balance,
                        price_in_sol,
                        price_in_usdc
                    }]
                    .endpoint(
                        move |bot: Bot, msg: Message, state: State, dialogue: MyDialogue| {
                            let services = services_for_dialog14.clone();
                            async move {
                                trade::receive_sell_amount(bot, msg, state, dialogue, services)
                                    .await
                            }
                        },
                    ),
                )
                .branch(
                    case![State::AwaitingSellConfirmation {
                        token_address,
                        token_symbol,
                        amount,
                        price_in_sol,
                        total_sol,
                        total_usdc
                    }]
                    .endpoint(
                        move |bot: Bot, msg: Message, state: State, dialogue: MyDialogue| {
                            let services = services_for_dialog15.clone();
                            async move {
                                trade::receive_sell_confirmation(
                                    bot, msg, state, dialogue, services,
                                )
                                .await
                            }
                        },
                    ),
                )
                .branch(case![State::AwaitingBuyManualAddress].endpoint(
                    move |bot: Bot, msg: Message, dialogue: MyDialogue| {
                        let services = services_for_dialog16.clone();
                        async move {
                            trade::receive_buy_manual_address(bot, msg, dialogue, services).await
                        }
                    },
                ))
                .branch(
                    case![State::AwaitingBuyAmount {
                        token_address,
                        token_symbol,
                        price_in_sol,
                        price_in_usdc
                    }]
                    .endpoint(
                        move |bot: Bot, msg: Message, state: State, dialogue: MyDialogue| {
                            let services = services_for_dialog17.clone();
                            async move {
                                trade::receive_buy_amount(bot, msg, state, dialogue, services).await
                            }
                        },
                    ),
                )
                .branch(
                    case![State::AwaitingBuyConfirmation {
                        token_address,
                        token_symbol,
                        amount,
                        price_in_sol,
                        total_sol,
                        total_usdc
                    }]
                    .endpoint(
                        move |bot: Bot, msg: Message, state: State, dialogue: MyDialogue| {
                            let services = services_for_dialog18.clone();
                            async move {
                                trade::receive_buy_confirmation(bot, msg, state, dialogue, services)
                                    .await
                            }
                        },
                    ),
                ),
        );

        // Add callback query handler for our buttons
        let callback_handler = Update::filter_callback_query().endpoint(
            move |bot: Bot, q: CallbackQuery, dialogue: MyDialogue| {
                let services = services_for_callbacks.clone();
                async move { handle_callback(bot, q, dialogue, services).await }
            },
        );

        teloxide::dispatching::dialogue::enter::<Update, InMemStorage<State>, State, _>()
            .branch(message_handler)
            .branch(callback_handler)
    }
}
