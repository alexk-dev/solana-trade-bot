use crate::interactor::watchlist_interactor::WatchlistInteractor;
use crate::solana::jupiter::price_service::PriceService;
use crate::view::watchlist_view::WatchlistView;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait WatchlistPresenter: Send + Sync {
    async fn show_watchlist(&self, telegram_id: i64) -> Result<()>;
    async fn show_token_detail(&self, telegram_id: i64, token_address: &str) -> Result<()>;
    async fn add_to_watchlist(&self, telegram_id: i64, token_address: &str) -> Result<()>;
    async fn remove_from_watchlist(&self, telegram_id: i64, token_address: &str) -> Result<()>;
    async fn prompt_for_token_address(&self) -> Result<()>;
    async fn refresh_watchlist(&self, telegram_id: i64) -> Result<()>;
}

pub struct WatchlistPresenterImpl<I, V> {
    interactor: Arc<I>,
    view: Arc<V>,
    price_service: Arc<dyn PriceService + Send + Sync>,
}

impl<I, V> WatchlistPresenterImpl<I, V>
where
    I: WatchlistInteractor,
    V: WatchlistView,
{
    pub fn new(
        interactor: Arc<I>,
        view: Arc<V>,
        price_service: Arc<dyn PriceService + Send + Sync>,
    ) -> Self {
        Self {
            interactor,
            view,
            price_service,
        }
    }
}

#[async_trait]
impl<I, V> WatchlistPresenter for WatchlistPresenterImpl<I, V>
where
    I: WatchlistInteractor + Send + Sync,
    V: WatchlistView + Send + Sync,
{
    async fn show_watchlist(&self, telegram_id: i64) -> Result<()> {
        match self.interactor.get_watchlist(telegram_id).await {
            Ok(watchlist) => {
                self.view.display_watchlist(watchlist).await?;
            }
            Err(e) => {
                self.view.display_error(e.to_string()).await?;
            }
        }

        Ok(())
    }

    async fn show_token_detail(&self, telegram_id: i64, token_address: &str) -> Result<()> {
        match self
            .interactor
            .get_watchlist_item(telegram_id, token_address)
            .await
        {
            Ok(Some(item)) => {
                // Get USDC price in addition to SOL price
                let price_in_usdc = match self.price_service.get_token_price(token_address).await {
                    Ok(price_info) => Some(price_info.price_in_usdc),
                    Err(_) => None,
                };

                self.view.display_token_detail(item, price_in_usdc).await?;
            }
            Ok(None) => {
                self.view
                    .display_error("Token not found in watchlist".to_string())
                    .await?;
            }
            Err(e) => {
                self.view.display_error(e.to_string()).await?;
            }
        }

        Ok(())
    }

    async fn add_to_watchlist(&self, telegram_id: i64, token_address: &str) -> Result<()> {
        match self
            .interactor
            .add_to_watchlist(telegram_id, token_address)
            .await
        {
            Ok(item) => {
                self.view.display_token_added(item).await?;
            }
            Err(e) => {
                self.view
                    .display_invalid_token_address(e.to_string())
                    .await?;
            }
        }

        Ok(())
    }

    async fn remove_from_watchlist(&self, telegram_id: i64, token_address: &str) -> Result<()> {
        // First get the token symbol for the confirmation message
        let token_symbol = match self
            .interactor
            .get_watchlist_item(telegram_id, token_address)
            .await
        {
            Ok(Some(item)) => item.token_symbol,
            _ => "Token".to_string(),
        };

        // Remove the token
        match self
            .interactor
            .remove_from_watchlist(telegram_id, token_address)
            .await
        {
            Ok(true) => {
                self.view.display_token_removed(&token_symbol).await?;
            }
            Ok(false) => {
                self.view
                    .display_error("Token not found in watchlist".to_string())
                    .await?;
            }
            Err(e) => {
                self.view.display_error(e.to_string()).await?;
            }
        }

        Ok(())
    }

    async fn prompt_for_token_address(&self) -> Result<()> {
        self.view.prompt_for_token_address().await
    }

    async fn refresh_watchlist(&self, telegram_id: i64) -> Result<()> {
        match self.interactor.refresh_watchlist_prices(telegram_id).await {
            Ok(watchlist) => {
                self.view.display_watchlist(watchlist).await?;
            }
            Err(e) => {
                self.view.display_error(e.to_string()).await?;
            }
        }

        Ok(())
    }
}
