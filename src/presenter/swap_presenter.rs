use crate::interactor::swap_interactor::SwapInteractor;
use crate::view::swap_view::SwapView;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait SwapPresenter: Send + Sync {
    async fn process_swap_command(&self, telegram_id: i64, command_parts: Vec<&str>) -> Result<()>;
}

pub struct SwapPresenterImpl<I, V> {
    interactor: Arc<I>,
    view: Arc<V>,
}

impl<I, V> SwapPresenterImpl<I, V>
where
    I: SwapInteractor,
    V: SwapView,
{
    pub fn new(interactor: Arc<I>, view: Arc<V>) -> Self {
        Self { interactor, view }
    }
}

#[async_trait]
impl<I, V> SwapPresenter for SwapPresenterImpl<I, V>
where
    I: SwapInteractor + Send + Sync,
    V: SwapView + Send + Sync,
{
    async fn process_swap_command(&self, telegram_id: i64, command_parts: Vec<&str>) -> Result<()> {
        if command_parts.len() < 4 {
            // Not enough parameters
            self.view.display_usage().await?;
            return Ok(());
        }

        let amount_str = command_parts[1];
        let source_token = command_parts[2];
        let target_token = command_parts[3];

        // Parse slippage (optional)
        let slippage_str = if command_parts.len() >= 5 {
            Some(command_parts[4])
        } else {
            None
        };

        // Validate swap parameters
        match self
            .interactor
            .validate_swap_parameters(amount_str, source_token, target_token, slippage_str)
            .await
        {
            Ok((amount, source_token, target_token, slippage)) => {
                // Show processing message
                let message = self
                    .view
                    .display_processing(&source_token, &target_token, amount)
                    .await?;

                // Execute swap
                let result = self
                    .interactor
                    .execute_swap(telegram_id, amount, &source_token, &target_token, slippage)
                    .await?;

                if result.success {
                    // Swap successful
                    self.view
                        .display_swap_success(
                            &result.source_token,
                            &result.target_token,
                            result.amount_in,
                            result.amount_out,
                            result.signature.as_deref().unwrap_or("unknown"),
                            message,
                        )
                        .await?;
                } else {
                    // Swap failed
                    self.view
                        .display_swap_error(
                            &result.source_token,
                            &result.target_token,
                            result.amount_in,
                            result
                                .error_message
                                .unwrap_or_else(|| "Unknown error".to_string()),
                            message,
                        )
                        .await?;
                }
            }
            Err(e) => {
                self.view.display_validation_error(e.to_string()).await?;
            }
        }

        Ok(())
    }
}
