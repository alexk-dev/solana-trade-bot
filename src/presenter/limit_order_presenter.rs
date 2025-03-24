// ./src/presenter/limit_order_presenter.rs
use crate::entity::LimitOrderType;
use crate::interactor::limit_order_interactor::LimitOrderInteractor;
use crate::view::limit_order_view::LimitOrderView;
use anyhow::Result;
use async_trait::async_trait;
use log::info;
use std::sync::Arc;

#[async_trait]
pub trait LimitOrderPresenter: Send + Sync {
    async fn show_limit_orders(&self, telegram_id: i64) -> Result<()>;
    async fn start_create_order_flow(&self) -> Result<()>;
    async fn handle_order_type_selection(&self, order_type: LimitOrderType) -> Result<()>;
    async fn handle_token_address(
        &self,
        address_text: &str,
        order_type: &LimitOrderType,
    ) -> Result<()>;
    async fn handle_price_and_amount(
        &self,
        price_amount_text: &str,
        order_type: &LimitOrderType,
        token_address: &str,
        token_symbol: &str,
        telegram_id: i64,
    ) -> Result<()>;
    async fn handle_confirmation(
        &self,
        confirmation_text: &str,
        order_type: &LimitOrderType,
        token_address: &str,
        token_symbol: &str,
        price_in_sol: f64,
        amount: f64,
        total_sol: f64,
        telegram_id: i64,
    ) -> Result<()>;
    async fn cancel_order(&self, order_id: i32) -> Result<()>;
}

pub struct LimitOrderPresenterImpl<I, V> {
    interactor: Arc<I>,
    view: Arc<V>,
}

impl<I, V> LimitOrderPresenterImpl<I, V>
where
    I: LimitOrderInteractor,
    V: LimitOrderView,
{
    pub fn new(interactor: Arc<I>, view: Arc<V>) -> Self {
        Self { interactor, view }
    }
}

#[async_trait]
impl<I, V> LimitOrderPresenter for LimitOrderPresenterImpl<I, V>
where
    I: LimitOrderInteractor + Send + Sync,
    V: LimitOrderView + Send + Sync,
{
    async fn show_limit_orders(&self, telegram_id: i64) -> Result<()> {
        info!("Fetching limit orders for user: {}", telegram_id);

        match self.interactor.get_active_limit_orders(telegram_id).await {
            Ok(orders) => {
                self.view.display_limit_orders(orders).await?;
            }
            Err(e) => {
                self.view.display_error(e.to_string()).await?;
            }
        }

        Ok(())
    }

    async fn start_create_order_flow(&self) -> Result<()> {
        info!("Starting limit order creation flow");
        self.view.prompt_for_order_type().await
    }

    async fn handle_order_type_selection(&self, order_type: LimitOrderType) -> Result<()> {
        info!("Selected order type: {:?}", order_type);
        self.view.prompt_for_token_address(&order_type).await
    }

    async fn handle_token_address(
        &self,
        address_text: &str,
        order_type: &LimitOrderType,
    ) -> Result<()> {
        info!("Processing token address: {}", address_text);

        if self.interactor.validate_token_address(address_text).await? {
            // Get token information to display to the user
            match self.interactor.get_token_info(address_text).await {
                Ok((token_symbol, price_in_sol, price_in_usdc)) => {
                    self.view
                        .display_token_info(
                            order_type,
                            address_text,
                            &token_symbol,
                            price_in_sol,
                            price_in_usdc,
                        )
                        .await?;
                    Ok(())
                }
                Err(e) => {
                    self.view
                        .display_error(format!("Error getting token info: {}", e))
                        .await?;
                    Ok(())
                }
            }
        } else {
            self.view.display_invalid_token_address().await?;
            Ok(())
        }
    }

    async fn handle_price_and_amount(
        &self,
        price_amount_text: &str,
        order_type: &LimitOrderType,
        token_address: &str,
        token_symbol: &str,
        telegram_id: i64,
    ) -> Result<()> {
        info!("Processing price and amount: {}", price_amount_text);

        match self
            .interactor
            .validate_order_price_and_amount(
                price_amount_text,
                order_type,
                token_address,
                token_symbol,
                telegram_id,
            )
            .await
        {
            Ok((price, amount, total_sol)) => {
                // Prompt for confirmation
                self.view
                    .prompt_for_confirmation(
                        order_type,
                        token_address,
                        token_symbol,
                        price,
                        amount,
                        total_sol,
                    )
                    .await?;
                Ok(())
            }
            Err(e) => {
                self.view
                    .display_invalid_price_amount(e.to_string())
                    .await?;
                Ok(())
            }
        }
    }

    async fn handle_confirmation(
        &self,
        confirmation_text: &str,
        order_type: &LimitOrderType,
        token_address: &str,
        token_symbol: &str,
        price_in_sol: f64,
        amount: f64,
        total_sol: f64,
        telegram_id: i64,
    ) -> Result<()> {
        let confirmation = confirmation_text.to_lowercase();

        if confirmation == "yes" || confirmation == "y" {
            info!(
                "Creating limit order: {:?} {} {} @ {}",
                order_type, amount, token_symbol, price_in_sol
            );

            // Create the order
            let result = self
                .interactor
                .create_limit_order(
                    telegram_id,
                    order_type,
                    token_address,
                    token_symbol,
                    price_in_sol,
                    amount,
                )
                .await?;

            if result.success {
                if let Some(order_id) = result.order_id {
                    self.view
                        .display_order_creation_success(
                            order_type,
                            token_symbol,
                            price_in_sol,
                            amount,
                            order_id,
                        )
                        .await?;
                } else {
                    self.view
                        .display_order_creation_error(
                            order_type,
                            token_symbol,
                            "Unknown error".to_string(),
                        )
                        .await?;
                }
            } else {
                self.view
                    .display_order_creation_error(
                        order_type,
                        token_symbol,
                        result
                            .error_message
                            .unwrap_or_else(|| "Unknown error".to_string()),
                    )
                    .await?;
            }
        } else {
            // Order cancelled
            self.view.display_order_cancelled().await?;
        }

        Ok(())
    }

    async fn cancel_order(&self, order_id: i32) -> Result<()> {
        info!("Cancelling order: {}", order_id);

        match self.interactor.cancel_limit_order(order_id).await {
            Ok(true) => {
                self.view.display_order_cancelled().await?;
                Ok(())
            }
            Ok(false) => {
                self.view
                    .display_error("Failed to cancel order".to_string())
                    .await?;
                Ok(())
            }
            Err(e) => {
                self.view
                    .display_error(format!("Error cancelling order: {}", e))
                    .await?;
                Ok(())
            }
        }
    }
}
