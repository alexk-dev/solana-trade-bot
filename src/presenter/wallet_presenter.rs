use crate::interactor::wallet_interactor::WalletInteractor;
use crate::view::wallet_view::WalletView;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait WalletPresenter: Send + Sync {
    async fn create_wallet(&self, telegram_id: i64) -> Result<()>;
    async fn show_wallet_address(&self, telegram_id: i64) -> Result<()>;
}

pub struct WalletPresenterImpl<I, V> {
    interactor: Arc<I>,
    view: Arc<V>,
}

impl<I, V> WalletPresenterImpl<I, V>
where
    I: WalletInteractor,
    V: WalletView,
{
    pub fn new(interactor: Arc<I>, view: Arc<V>) -> Self {
        Self { interactor, view }
    }
}

#[async_trait]
impl<I, V> WalletPresenter for WalletPresenterImpl<I, V>
where
    I: WalletInteractor + Send + Sync,
    V: WalletView + Send + Sync,
{
    async fn create_wallet(&self, telegram_id: i64) -> Result<()> {
        match self.interactor.create_wallet(telegram_id).await {
            Ok((mnemonic, _keypair, address)) => {
                self.view.display_wallet_created(address, mnemonic).await?;
                Ok(())
            }
            Err(e) => {
                if let Some(wallet_error) = e.downcast_ref::<crate::entity::BotError>() {
                    match wallet_error {
                        crate::entity::BotError::WalletCreationError(_) => {
                            self.view.display_wallet_already_exists().await?;
                        }
                        _ => {
                            self.view.display_error(e.to_string()).await?;
                        }
                    }
                } else {
                    self.view.display_error(e.to_string()).await?;
                }
                Ok(())
            }
        }
    }

    async fn show_wallet_address(&self, telegram_id: i64) -> Result<()> {
        match self.interactor.get_wallet_info(telegram_id).await? {
            Some((address, _mnemonic)) => {
                self.view.display_wallet_address(address).await?;
                Ok(())
            }
            None => {
                self.view.display_no_wallet().await?;
                Ok(())
            }
        }
    }
}
