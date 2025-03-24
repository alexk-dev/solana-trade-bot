use anyhow::Result;
use log::info;
use std::sync::Arc;
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode},
};

use super::{ui, CommandHandler, MyDialogue};
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
        telegram_id: i64,
        _dialogue: Option<MyDialogue>,
        services: Arc<ServiceContainer>,
    ) -> Result<()> {
        let chat_id = msg.chat.id;

        info!(
            "Create wallet command received from Telegram ID: {}",
            telegram_id
        );

        let db_pool = services.db_pool();
        let interactor = Arc::new(WalletInteractorImpl::new(db_pool));
        let view = Arc::new(TelegramWalletView::new(bot.clone(), chat_id));
        let presenter = WalletPresenterImpl::new(interactor, view);

        let result = presenter.create_wallet(telegram_id).await;

        // After creating wallet, show the main menu again with buttons
        if result.is_ok() {
            // Show user the main menu
            let keyboard = ui::create_wallet_menu_keyboard();
            bot.send_message(
                chat_id,
                "Your wallet has been created successfully. What would you like to do next?",
            )
            .reply_markup(keyboard)
            .await?;
        }

        Ok(())
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
        telegram_id: i64,
        _dialogue: Option<MyDialogue>,
        services: Arc<ServiceContainer>,
    ) -> Result<()> {
        let chat_id = msg.chat.id;

        info!("Address command received from Telegram ID: {}", telegram_id);

        let db_pool = services.db_pool();
        let interactor = Arc::new(WalletInteractorImpl::new(db_pool));
        let view = Arc::new(TelegramWalletView::new(bot.clone(), chat_id));
        let presenter = WalletPresenterImpl::new(interactor, view);

        // Show address with QR code
        presenter.show_wallet_address(telegram_id).await;

        Ok(())
    }
}
