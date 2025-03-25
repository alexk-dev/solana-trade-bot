use anyhow::{anyhow, Result};
use log::info;
use std::sync::Arc;
use teloxide::{prelude::*, types::ParseMode};

use super::{CommandHandler, MyDialogue};
use crate::di::ServiceContainer;
use crate::interactor::balance_interactor::BalanceInteractorImpl;
use crate::interactor::db;
use crate::interactor::wallet_interactor::WalletInteractorImpl;
use crate::presenter::balance_presenter::{BalancePresenter, BalancePresenterImpl};
use crate::presenter::wallet_presenter::{WalletPresenter, WalletPresenterImpl};
use crate::view::balance_view::TelegramBalanceView;
use crate::view::wallet_view::TelegramWalletView;

pub struct StartCommand;

impl CommandHandler for StartCommand {
    fn command_name() -> &'static str {
        "start"
    }

    fn description() -> &'static str {
        "start the bot"
    }

    async fn execute(
        bot: Bot,
        msg: Message,
        telegram_id: i64,
        _dialogue: Option<MyDialogue>,
        services: Arc<ServiceContainer>,
    ) -> Result<()> {
        let db_pool = services.db_pool();
        let username = msg.from().and_then(|user| user.username.clone());
        let chat_id = msg.chat.id;

        info!("Start command received from Telegram ID: {}", telegram_id);

        let user_exists = db::check_user_exists(&db_pool, telegram_id)
            .await
            .map_err(|e| anyhow!("Database error: {}", e))?;

        // Register user if they don't exist
        if !user_exists {
            db::create_user(&db_pool, telegram_id, username)
                .await
                .map_err(|e| anyhow!("Failed to create user: {}", e))?;

            bot.send_message(
                chat_id,
                "<b>Hello!</b> I'm a Solana trading bot. You have been successfully registered.",
            )
            .parse_mode(ParseMode::Html)
            .await?;
        } else {
            // Welcome returning user
            bot.send_message(chat_id, "<b>Welcome back to Solana Trading Bot!</b>")
                .parse_mode(ParseMode::Html)
                .await?;
        }

        // Check if user has a wallet and create one if not
        let user = db::get_user_by_telegram_id(&db_pool, telegram_id).await?;

        if user.solana_address.is_none() {
            info!(
                "User {} does not have a wallet. Creating one automatically.",
                telegram_id
            );

            // Create wallet interactor and presenter
            let wallet_interactor = Arc::new(WalletInteractorImpl::new(db_pool.clone()));
            let wallet_view = Arc::new(TelegramWalletView::new(bot.clone(), chat_id));
            let wallet_presenter = WalletPresenterImpl::new(wallet_interactor, wallet_view);

            // Create wallet
            match wallet_presenter.create_wallet(telegram_id).await {
                Ok(()) => {
                    bot.send_message(
                        chat_id,
                        "I've automatically created a Solana wallet for you! ✅\nYou can now send and receive tokens.",
                    ).await?;
                }
                Err(e) => {
                    info!("Failed to auto-create wallet: {}", e);
                    // Continue without wallet - will show balance page with option to create wallet
                }
            }
        }

        // Display balance (or no wallet message)
        let solana_client = services.solana_client();
        let price_service = services.price_service();
        let interactor = Arc::new(BalanceInteractorImpl::new(
            db_pool.clone(),
            solana_client,
            price_service,
        ));
        let view = Arc::new(TelegramBalanceView::new(bot, chat_id));
        let presenter = BalancePresenterImpl::new(interactor, view);

        presenter.show_balances(telegram_id).await?;

        Ok(())
    }
}
