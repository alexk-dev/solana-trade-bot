use crate::entity::OrderType;
use crate::interactor::trade_interactor::TradeInteractor;
use crate::view::trade_view::TradeView;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait TradePresenter: Send + Sync {
    async fn start_trade_flow(&self, trade_type: &OrderType) -> Result<()>;
    async fn handle_token_address(&self, address_text: &str, trade_type: &OrderType) -> Result<()>;
    async fn handle_confirmation(
        &self,
        confirmation_text: &str,
        trade_type: &OrderType,
        token_address: &str,
        token_symbol: &str,
        amount: f64,
        price_in_sol: f64,
        total_sol: f64,
        telegram_id: i64,
    ) -> Result<()>;
}

pub struct TradePresenterImpl<I, V> {
    interactor: Arc<I>,
    view: Arc<V>,
}

impl<I, V> TradePresenterImpl<I, V>
where
    I: TradeInteractor,
    V: TradeView,
{
    pub fn new(interactor: Arc<I>, view: Arc<V>) -> Self {
        Self { interactor, view }
    }
}

#[async_trait]
impl<I, V> TradePresenter for TradePresenterImpl<I, V>
where
    I: TradeInteractor + Send + Sync,
    V: TradeView + Send + Sync,
{
    async fn start_trade_flow(&self, trade_type: &OrderType) -> Result<()> {
        self.view.prompt_for_token_address(trade_type).await
    }

    async fn handle_token_address(&self, address_text: &str, trade_type: &OrderType) -> Result<()> {
        if self.interactor.validate_token_address(address_text).await? {
            // Get token information to display to the user
            match self.interactor.get_token_info(address_text).await {
                Ok((token_symbol, price_in_sol, price_in_usdc)) => {
                    self.view
                        .display_token_info(
                            trade_type,
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

    async fn handle_confirmation(
        &self,
        confirmation_text: &str,
        trade_type: &OrderType,
        token_address: &str,
        token_symbol: &str,
        amount: f64,
        price_in_sol: f64,
        total_sol: f64,
        telegram_id: i64,
    ) -> Result<()> {
        let confirmation = confirmation_text.to_lowercase();

        if confirmation == "yes" || confirmation == "y" {
            // Show processing message
            let message = self.view.display_processing(trade_type).await?;

            // Execute the trade
            let result = self
                .interactor
                .execute_trade(
                    telegram_id,
                    trade_type,
                    token_address,
                    token_symbol,
                    amount,
                    price_in_sol,
                )
                .await?;

            if result.success {
                self.view
                    .display_trade_success(
                        trade_type,
                        token_symbol,
                        amount,
                        price_in_sol,
                        total_sol,
                        result.signature.as_deref().unwrap_or("unknown"),
                        message,
                    )
                    .await?;
            } else {
                self.view
                    .display_trade_error(
                        trade_type,
                        token_symbol,
                        amount,
                        result
                            .error_message
                            .unwrap_or_else(|| "Unknown error".to_string()),
                        message,
                    )
                    .await?;
            }
        } else {
            // Trade cancelled
            self.view.display_trade_cancelled().await?;
        }

        Ok(())
    }
}
