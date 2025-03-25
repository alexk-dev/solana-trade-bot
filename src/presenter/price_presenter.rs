use crate::interactor::price_interactor::PriceInteractor;
use crate::view::price_view::PriceView;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait PricePresenter: Send + Sync {
    async fn show_token_price(&self, token_id: &str) -> Result<()>;
}

pub struct PricePresenterImpl<I, V> {
    interactor: Arc<I>,
    view: Arc<V>,
}

impl<I, V> PricePresenterImpl<I, V>
where
    I: PriceInteractor,
    V: PriceView,
{
    pub fn new(interactor: Arc<I>, view: Arc<V>) -> Self {
        Self { interactor, view }
    }
}

#[async_trait]
impl<I, V> PricePresenter for PricePresenterImpl<I, V>
where
    I: PriceInteractor + Send + Sync,
    V: PriceView + Send + Sync,
{
    async fn show_token_price(&self, token_id: &str) -> Result<()> {
        self.view.display_loading(token_id).await?;

        match self.interactor.get_token_price(token_id).await {
            Ok(price_info) => {
                self.view
                    .display_price(
                        &price_info.token_id,
                        &price_info.symbol,
                        price_info.price_in_sol,
                        price_info.price_in_usdc,
                    )
                    .await?;
            }
            Err(e) => {
                self.view.display_error(e.to_string()).await?;
            }
        }

        Ok(())
    }
}
