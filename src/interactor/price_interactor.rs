use crate::entity::TokenPrice;
use crate::solana::jupiter::PriceService;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait PriceInteractor: Send + Sync {
    async fn get_token_price(&self, token_id: &str) -> Result<TokenPrice>;
}

pub struct PriceInteractorImpl {
    price_service: Arc<dyn PriceService + Send + Sync>,
}

impl PriceInteractorImpl {
    pub fn new(price_service: Arc<dyn PriceService + Send + Sync>) -> Self {
        Self { price_service }
    }
}

#[async_trait]
impl PriceInteractor for PriceInteractorImpl {
    async fn get_token_price(&self, token_id: &str) -> Result<TokenPrice> {
        self.price_service.get_token_price(token_id).await
    }
}
