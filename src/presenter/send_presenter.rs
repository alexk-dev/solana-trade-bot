use crate::interactor::send_interactor::SendInteractor;
use crate::view::send_view::SendView;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait SendPresenter: Send + Sync {
    async fn start_send_flow(&self) -> Result<()>;
    async fn handle_recipient_address(&self, address_text: &str) -> Result<()>;
    async fn handle_amount(&self, amount_text: &str, recipient: &str) -> Result<()>;
    async fn handle_confirmation(
        &self,
        confirmation_text: &str,
        recipient: &str,
        amount: f64,
        token: &str,
        telegram_id: i64,
    ) -> Result<()>;

    async fn validate_address(&self, address: &str) -> Result<bool>;
}

pub struct SendPresenterImpl<I, V> {
    interactor: Arc<I>,
    view: Arc<V>,
}
impl<I, V> SendPresenterImpl<I, V>
where
    I: SendInteractor,
    V: SendView,
{
    pub fn new(interactor: Arc<I>, view: Arc<V>) -> Self {
        Self { interactor, view }
    }
}

#[async_trait]
impl<I, V> SendPresenter for SendPresenterImpl<I, V>
where
    I: SendInteractor + Send + Sync,
    V: SendView + Send + Sync,
{
    async fn start_send_flow(&self) -> Result<()> {
        self.view.prompt_for_recipient_address().await?;
        Ok(())
    }

    async fn handle_recipient_address(&self, address_text: &str) -> Result<()> {
        if self.interactor.validate_address(address_text).await? {
            self.view.prompt_for_amount().await?;
            Ok(())
        } else {
            self.view.display_invalid_address().await?;
            Ok(())
        }
    }

    async fn handle_amount(&self, amount_text: &str, recipient: &str) -> Result<()> {
        match self.interactor.parse_amount_and_token(amount_text).await {
            Ok((amount, token)) => {
                self.view
                    .prompt_for_confirmation(recipient, amount, &token)
                    .await?;
                Ok(())
            }
            Err(e) => {
                self.view.display_invalid_amount(e.to_string()).await?;
                Ok(())
            }
        }
    }

    async fn handle_confirmation(
        &self,
        confirmation_text: &str,
        recipient: &str,
        amount: f64,
        token: &str,
        telegram_id: i64,
    ) -> Result<()> {
        let confirmation = confirmation_text.to_lowercase();

        if confirmation == "yes" {
            // Show "processing" message
            let message = self.view.display_processing().await?;

            // Execute the transaction
            let result = self
                .interactor
                .send_transaction(telegram_id, recipient, amount, token)
                .await?;

            if result.success {
                self.view
                    .display_transaction_success(
                        &result.recipient,
                        result.amount,
                        &result.token,
                        result.signature.as_deref().unwrap_or("unknown"),
                        message,
                    )
                    .await?;
            } else {
                self.view
                    .display_transaction_error(
                        &result.recipient,
                        result.amount,
                        &result.token,
                        result
                            .error_message
                            .unwrap_or_else(|| "Unknown error".to_string()),
                        message,
                    )
                    .await?;
            }
        } else {
            // Transaction cancelled
            self.view.display_transaction_cancelled().await?;
        }

        Ok(())
    }

    async fn validate_address(&self, address: &str) -> Result<bool> {
        self.interactor.validate_address(address).await
    }
}
