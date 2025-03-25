use crate::entity::BotError;
use crate::interactor::balance_interactor::BalanceInteractor;
use crate::view::balance_view::BalanceView;
use anyhow::Result;
use async_trait::async_trait;
use log::info;
use std::sync::Arc;
use teloxide::types::Message;

#[async_trait]
pub trait BalancePresenter: Send + Sync {
    // Original method
    async fn show_balances(&self, telegram_id: i64) -> Result<()>;

    // New method for refreshing balance with existing message
    async fn refresh_balances(&self, telegram_id: i64, message: Option<Message>) -> Result<()>;
}

pub struct BalancePresenterImpl<I, V> {
    interactor: Arc<I>,
    view: Arc<V>,
}

impl<I, V> BalancePresenterImpl<I, V>
where
    I: BalanceInteractor,
    V: BalanceView,
{
    pub fn new(interactor: Arc<I>, view: Arc<V>) -> Self {
        Self { interactor, view }
    }
}

#[async_trait]
impl<I, V> BalancePresenter for BalancePresenterImpl<I, V>
where
    I: BalanceInteractor + Send + Sync,
    V: BalanceView + Send + Sync,
{
    // Original implementation
    async fn show_balances(&self, telegram_id: i64) -> Result<()> {
        let message = self.view.display_loading().await?;

        match self.interactor.get_wallet_balances(telegram_id).await {
            Ok((address, sol_balance, token_balances, usd_values)) => {
                // Calculate total USD value
                let total_usd: f64 = usd_values.iter().map(|(_, value)| value).sum();

                self.view
                    .display_balances(
                        address,
                        sol_balance,
                        token_balances,
                        usd_values,
                        total_usd,
                        message,
                    )
                    .await?;
            }
            Err(e) => {
                if let Some(wallet_error) = e.downcast_ref::<BotError>() {
                    match wallet_error {
                        BotError::WalletNotFound => {
                            self.view.display_no_wallet(message).await?;
                        }
                        _ => {
                            self.view.display_error(e.to_string(), message).await?;
                        }
                    }
                } else {
                    self.view.display_error(e.to_string(), message).await?;
                }
            }
        }

        Ok(())
    }

    // New implementation for refresh
    async fn refresh_balances(&self, telegram_id: i64, message: Option<Message>) -> Result<()> {
        info!("Refreshing balances for user: {}", telegram_id);

        // Show loading message or update existing one
        let loading_message = if let Some(msg) = message {
            self.view.display_loading_update(msg).await?
        } else {
            self.view.display_loading().await?
        };

        // Get wallet balances from interactor
        match self.interactor.get_wallet_balances(telegram_id).await {
            Ok((address, sol_balance, token_balances, usd_values)) => {
                // Calculate total USD value
                let total_usd: f64 = usd_values.iter().map(|(_, value)| value).sum();

                // Display balances using view
                self.view
                    .display_balances(
                        address,
                        sol_balance,
                        token_balances,
                        usd_values,
                        total_usd,
                        loading_message,
                    )
                    .await?;
            }
            Err(e) => {
                // Handle errors
                if let Some(wallet_error) = e.downcast_ref::<BotError>() {
                    match wallet_error {
                        BotError::WalletNotFound => {
                            self.view.display_no_wallet(loading_message).await?;
                        }
                        _ => {
                            self.view
                                .display_error(e.to_string(), loading_message)
                                .await?;
                        }
                    }
                } else {
                    self.view
                        .display_error(e.to_string(), loading_message)
                        .await?;
                }
            }
        }

        Ok(())
    }
}
