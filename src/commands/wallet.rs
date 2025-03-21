use anyhow::Result;
use log::info;
use std::sync::Arc;
use teloxide::prelude::*;

use super::{CommandHandler, MyDialogue};
use crate::di::ServiceContainer;
use crate::interactor::wallet_interactor::WalletInteractorImpl;
use crate::presenter::wallet_presenter::{WalletPresenter, WalletPresenterImpl};
use crate::view::wallet_view::TelegramWalletView;

pub struct CreateWalletCommand;

impl CommandHandler for CreateWalletCommand {
    fn command_name() -> &'static str {
        "create_wallet"
    }

    fn description() -> &'static str {
        "create a new Solana wallet"
    }

    async fn execute(
        bot: Bot,
        msg: Message,
        _dialogue: Option<MyDialogue>,
        services: Arc<ServiceContainer>,
    ) -> Result<()> {
        let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);
        let chat_id = msg.chat.id;

        info!(
            "Create wallet command received from Telegram ID: {}",
            telegram_id
        );

        let db_pool = services.db_pool();
        let interactor = Arc::new(WalletInteractorImpl::new(db_pool));
        let view = Arc::new(TelegramWalletView::new(bot, chat_id));
        let presenter = WalletPresenterImpl::new(interactor, view);

        // Execute the use case via presenter
        presenter.create_wallet(telegram_id).await
    }
}

pub struct AddressCommand;

impl CommandHandler for AddressCommand {
    fn command_name() -> &'static str {
        "address"
    }

    fn description() -> &'static str {
        "show your wallet address and QR code"
    }

    async fn execute(
        bot: Bot,
        msg: Message,
        _dialogue: Option<MyDialogue>,
        services: Arc<ServiceContainer>,
    ) -> Result<()> {
        let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);
        let chat_id = msg.chat.id;

        info!("Address command received from Telegram ID: {}", telegram_id);

        // Create VIPER components
        let db_pool = services.db_pool();
        let interactor = Arc::new(WalletInteractorImpl::new(db_pool));
        let view = Arc::new(TelegramWalletView::new(bot, chat_id));
        let presenter = WalletPresenterImpl::new(interactor, view);

        // Execute the use case via presenter
        presenter.show_wallet_address(telegram_id).await
    }
}
