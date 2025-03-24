use anyhow::Result;
use log::info;
use std::sync::Arc;
use teloxide::prelude::*;

use super::{CommandHandler, MyDialogue};
use crate::di::ServiceContainer;
use crate::entity::{LimitOrderType, State};
use crate::interactor::limit_order_interactor::{LimitOrderInteractor, LimitOrderInteractorImpl};
use crate::presenter::limit_order_presenter::{LimitOrderPresenter, LimitOrderPresenterImpl};
use crate::view::limit_order_view::TelegramLimitOrderView;

pub struct LimitOrdersCommand;

impl CommandHandler for LimitOrdersCommand {
    fn command_name() -> &'static str {
        "limit_orders"
    }

    fn description() -> &'static str {
        "manage your limit orders"
    }

    async fn execute(
        bot: Bot,
        msg: Message,
        telegram_id: i64,
        _dialogue: Option<MyDialogue>,
        services: Arc<ServiceContainer>,
    ) -> Result<()> {
        let chat_id = msg.chat.id;

        info!("Limit orders command initiated by user: {}", telegram_id);

        let db_pool = services.db_pool();
        let solana_client = services.solana_client();
        let price_service = services.price_service();
        let token_repository = services.token_repository();

        let interactor = Arc::new(LimitOrderInteractorImpl::new(
            db_pool,
            solana_client,
            price_service,
            token_repository,
        ));
        let view = Arc::new(TelegramLimitOrderView::new(bot, chat_id));
        let presenter = LimitOrderPresenterImpl::new(interactor, view);

        presenter.show_limit_orders(telegram_id).await?;

        Ok(())
    }
}

// Handler for the order type selection (via callback)
pub async fn handle_order_type_selection(
    bot: Bot,
    msg: Message,
    order_type: LimitOrderType,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    let chat_id = msg.chat.id;

    dialogue
        .update(State::AwaitingLimitOrderTokenAddress {
            order_type: order_type.clone(),
        })
        .await?;

    let db_pool = services.db_pool();
    let solana_client = services.solana_client();
    let price_service = services.price_service();
    let token_repository = services.token_repository();

    let interactor = Arc::new(LimitOrderInteractorImpl::new(
        db_pool,
        solana_client,
        price_service,
        token_repository,
    ));
    let view = Arc::new(TelegramLimitOrderView::new(bot, chat_id));
    let presenter = LimitOrderPresenterImpl::new(interactor, view);

    presenter.handle_order_type_selection(order_type).await?;

    Ok(())
}

// Handler for the token address state
pub async fn receive_token_address(
    bot: Bot,
    msg: Message,
    state: State,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    if let State::AwaitingLimitOrderTokenAddress { order_type } = state {
        if let Some(address_text) = msg.text() {
            let chat_id = msg.chat.id;

            let db_pool = services.db_pool();
            let solana_client = services.solana_client();
            let price_service = services.price_service();
            let token_repository = services.token_repository();

            let interactor = Arc::new(LimitOrderInteractorImpl::new(
                db_pool,
                solana_client.clone(),
                price_service.clone(),
                token_repository.clone(),
            ));
            let view = Arc::new(TelegramLimitOrderView::new(bot.clone(), chat_id));
            let presenter = LimitOrderPresenterImpl::new(interactor.clone(), view);

            // Validate token address
            if let Ok(is_valid) = interactor.validate_token_address(address_text).await {
                if is_valid {
                    // Get token info to show to the user
                    match interactor.get_token_info(address_text).await {
                        Ok((token_symbol, price_in_sol, price_in_usdc)) => {
                            // Update dialogue state
                            dialogue
                                .update(State::AwaitingLimitOrderPriceAndAmount {
                                    order_type: order_type.clone(),
                                    token_address: address_text.to_string(),
                                    token_symbol: token_symbol.clone(),
                                    current_price_in_sol: price_in_sol,
                                    current_price_in_usdc: price_in_usdc,
                                })
                                .await?;

                            presenter
                                .handle_token_address(address_text, &order_type)
                                .await?;
                        }
                        Err(e) => {
                            bot.send_message(chat_id, format!("Error getting token info: {}", e))
                                .await?;
                        }
                    }
                } else {
                    bot.send_message(
                        chat_id,
                        "Invalid token address. Please enter a valid Solana token contract address:",
                    )
                        .await?;
                }
            } else {
                bot.send_message(chat_id, "Error validating token address. Please try again:")
                    .await?;
            }
        } else {
            bot.send_message(
                msg.chat.id,
                "Please enter the token contract address as text:",
            )
            .await?;
        }
    }

    Ok(())
}

// Handler for price and amount state
pub async fn receive_price_and_amount(
    bot: Bot,
    msg: Message,
    state: State,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    if let State::AwaitingLimitOrderPriceAndAmount {
        order_type,
        token_address,
        token_symbol,
        current_price_in_sol,
        current_price_in_usdc,
    } = state
    {
        if let Some(price_amount_text) = msg.text() {
            let chat_id = msg.chat.id;
            let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);

            let db_pool = services.db_pool();
            let solana_client = services.solana_client();
            let price_service = services.price_service();
            let token_repository = services.token_repository();

            let interactor = Arc::new(LimitOrderInteractorImpl::new(
                db_pool,
                solana_client.clone(),
                price_service.clone(),
                token_repository.clone(),
            ));
            let view = Arc::new(TelegramLimitOrderView::new(bot.clone(), chat_id));
            let presenter = LimitOrderPresenterImpl::new(interactor.clone(), view);

            // Parse and validate price and amount
            match interactor
                .validate_order_price_and_amount(
                    price_amount_text,
                    &order_type,
                    &token_address,
                    &token_symbol,
                    telegram_id,
                )
                .await
            {
                Ok((price, amount, total_sol)) => {
                    // Update dialogue state
                    dialogue
                        .update(State::AwaitingLimitOrderConfirmation {
                            order_type: order_type.clone(),
                            token_address: token_address.clone(),
                            token_symbol: token_symbol.clone(),
                            price_in_sol: price,
                            amount,
                            total_sol,
                        })
                        .await?;

                    // Prompt for confirmation
                    bot.send_message(
                        chat_id,
                        format!(
                            "Please confirm your limit order:\n\n{} {} {} @ {:.6} SOL each\nTotal: {:.6} SOL\n\nDo you want to proceed? (yes/no)",
                            order_type, amount, token_symbol, price, total_sol
                        ),
                    )
                        .await?;
                }
                Err(e) => {
                    bot.send_message(chat_id, format!("Invalid input: {}", e))
                        .await?;
                }
            }
        } else {
            bot.send_message(
                msg.chat.id,
                "Please enter the price and amount in the format: <price> <amount>",
            )
            .await?;
        }
    }

    Ok(())
}

// Handler for confirmation state
pub async fn receive_confirmation(
    bot: Bot,
    msg: Message,
    state: State,
    dialogue: MyDialogue,
    services: Arc<ServiceContainer>,
) -> Result<()> {
    if let State::AwaitingLimitOrderConfirmation {
        order_type,
        token_address,
        token_symbol,
        price_in_sol,
        amount,
        total_sol,
    } = state
    {
        if let Some(text) = msg.text() {
            let confirmation_text = text.to_lowercase();
            let chat_id = msg.chat.id;
            let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);

            // Reset dialogue state
            dialogue.update(State::Start).await?;

            let db_pool = services.db_pool();
            let solana_client = services.solana_client();
            let price_service = services.price_service();
            let token_repository = services.token_repository();

            let interactor = Arc::new(LimitOrderInteractorImpl::new(
                db_pool,
                solana_client,
                price_service,
                token_repository,
            ));
            let view = Arc::new(TelegramLimitOrderView::new(bot, chat_id));
            let presenter = LimitOrderPresenterImpl::new(interactor, view);

            presenter
                .handle_confirmation(
                    &confirmation_text,
                    &order_type,
                    &token_address,
                    &token_symbol,
                    price_in_sol,
                    amount,
                    total_sol,
                    telegram_id,
                )
                .await?;
        } else {
            bot.send_message(msg.chat.id, "Please confirm with 'yes' or 'no' as text:")
                .await?;
        }
    }

    Ok(())
}
