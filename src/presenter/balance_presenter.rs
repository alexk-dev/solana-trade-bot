use crate::entity::BotError;
use crate::interactor::balance_interactor::BalanceInteractor;
use crate::view::balance_view::BalanceView;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait BalancePresenter: Send + Sync {
    async fn show_balances(&self, telegram_id: i64) -> Result<()>;
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
}
