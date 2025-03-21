use anyhow::Result;
use log::info;
use std::sync::Arc;
use teloxide::prelude::*;

use super::{CommandHandler, MyDialogue};
use crate::di::ServiceContainer;
use crate::interactor::balance_interactor::BalanceInteractorImpl;
use crate::presenter::balance_presenter::{BalancePresenter, BalancePresenterImpl};
use crate::view::balance_view::TelegramBalanceView;

pub struct BalanceCommand;

impl CommandHandler for BalanceCommand {
    fn command_name() -> &'static str {
        "balance"
    }

    fn description() -> &'static str {
        "check your wallet balance and token holdings"
    }

    async fn execute(
        bot: Bot,
        msg: Message,
        _dialogue: Option<MyDialogue>,
        services: Arc<ServiceContainer>,
    ) -> Result<()> {
        let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);
        let chat_id = msg.chat.id;

        info!("Balance command received from Telegram ID: {}", telegram_id);

        let db_pool = services.db_pool();
        let solana_client = services.solana_client();
        let price_service = services.price_service();

        // Create interactor
        let interactor = Arc::new(BalanceInteractorImpl::new(
            db_pool,
            solana_client,
            price_service,
        ));

        // Create view
        let view = Arc::new(TelegramBalanceView::new(bot, chat_id));

        // Create presenter
        let presenter = BalancePresenterImpl::new(interactor, view);

        // Execute the use case via presenter
        presenter.show_balances(telegram_id).await
    }
}
