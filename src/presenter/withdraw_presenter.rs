use crate::interactor::withdraw_interactor::WithdrawInteractor;
use crate::view::withdraw_view::WithdrawView;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait WithdrawPresenter: Send + Sync {
    async fn start_withdraw_flow(&self, telegram_id: i64) -> Result<()>;
    async fn show_token_details(&self, token_address: &str, telegram_id: i64) -> Result<()>;
    async fn handle_recipient_address(
        &self,
        address_text: &str,
        token_address: &str,
        token_symbol: &str,
        amount: f64,
        price_in_sol: f64,
        price_in_usdc: f64,
    ) -> Result<()>;
    async fn handle_amount_input(
        &self,
        amount_text: &str,
        token_address: &str,
        token_symbol: &str,
        recipient: &str,
        balance: f64,
        price_in_sol: f64,
        price_in_usdc: f64,
    ) -> Result<()>;
    async fn handle_confirmation(
        &self,
        confirmation_text: &str,
        token_address: &str,
        token_symbol: &str,
        recipient: &str,
        amount: f64,
        price_in_sol: f64,
        total_sol: f64,
        total_usdc: f64,
        telegram_id: i64,
    ) -> Result<()>;
}

pub struct WithdrawPresenterImpl<I, V> {
    interactor: Arc<I>,
    view: Arc<V>,
}

impl<I, V> WithdrawPresenterImpl<I, V>
where
    I: WithdrawInteractor,
    V: WithdrawView,
{
    pub fn new(interactor: Arc<I>, view: Arc<V>) -> Self {
        Self { interactor, view }
    }
}

#[async_trait]
impl<I, V> WithdrawPresenter for WithdrawPresenterImpl<I, V>
where
    I: WithdrawInteractor + Send + Sync,
    V: WithdrawView + Send + Sync,
{
    async fn start_withdraw_flow(&self, telegram_id: i64) -> Result<()> {
        match self.interactor.get_user_tokens(telegram_id).await {
            Ok(tokens) => {
                if tokens.is_empty() {
                    self.view.display_no_tokens().await?;
                } else {
                    self.view.display_token_selection(tokens).await?;
                }
                Ok(())
            }
            Err(e) => {
                if e.to_string().contains("Wallet not found") {
                    self.view.display_no_wallet().await?;
                } else {
                    self.view.display_error(e.to_string()).await?;
                }
                Ok(())
            }
        }
    }

    async fn show_token_details(&self, token_address: &str, telegram_id: i64) -> Result<()> {
        // Get token info and balance
        match self.interactor.get_user_tokens(telegram_id).await {
            Ok(tokens) => {
                let token = tokens.iter().find(|t| t.mint_address == token_address);

                if let Some(token_balance) = token {
                    // Get current token price
                    match self.interactor.get_token_price(token_address).await {
                        Ok((price_in_sol, price_in_usdc)) => {
                            // Show token details
                            self.view
                                .display_token_details(
                                    &token_balance.symbol,
                                    token_address,
                                    token_balance.amount,
                                    price_in_sol,
                                    price_in_usdc,
                                )
                                .await?;
                        }
                        Err(e) => {
                            self.view
                                .display_error(format!("Failed to get token price: {}", e))
                                .await?;
                        }
                    }
                } else {
                    self.view
                        .display_error(format!(
                            "Token with address {} not found in your wallet",
                            token_address
                        ))
                        .await?;
                }

                Ok(())
            }
            Err(e) => {
                self.view.display_error(e.to_string()).await?;
                Ok(())
            }
        }
    }

    async fn handle_recipient_address(
        &self,
        address_text: &str,
        token_address: &str,
        token_symbol: &str,
        amount: f64,
        price_in_sol: f64,
        price_in_usdc: f64,
    ) -> Result<()> {
        // Validate the recipient address
        if self
            .interactor
            .validate_recipient_address(address_text)
            .await?
        {
            // If valid, prompt for amount
            self.view
                .prompt_for_amount(token_symbol, amount, price_in_sol, price_in_usdc)
                .await?;
        } else {
            // If invalid, show error and prompt again
            self.view.display_invalid_address().await?;
        }

        Ok(())
    }

    async fn handle_amount_input(
        &self,
        amount_text: &str,
        token_address: &str,
        token_symbol: &str,
        recipient: &str,
        balance: f64,
        price_in_sol: f64,
        price_in_usdc: f64,
    ) -> Result<()> {
        // Validate amount
        match self
            .interactor
            .validate_withdraw_amount(amount_text, balance)
            .await
        {
            Ok(amount) => {
                // Calculate total values
                let total_sol = amount * price_in_sol;
                let total_usdc = amount * price_in_usdc;

                // Prompt for confirmation
                self.view
                    .prompt_for_confirmation(token_symbol, recipient, amount, total_sol, total_usdc)
                    .await?;
            }
            Err(e) => {
                self.view.display_invalid_amount(e.to_string()).await?;
            }
        }

        Ok(())
    }

    async fn handle_confirmation(
        &self,
        confirmation_text: &str,
        token_address: &str,
        token_symbol: &str,
        recipient: &str,
        amount: f64,
        price_in_sol: f64,
        total_sol: f64,
        total_usdc: f64,
        telegram_id: i64,
    ) -> Result<()> {
        let confirmation = confirmation_text.to_lowercase();

        if confirmation == "yes" || confirmation == "y" {
            // Show processing message
            let processing_message = self.view.display_processing().await?;

            // Execute the withdrawal
            let result = self
                .interactor
                .execute_withdraw(
                    telegram_id,
                    token_address,
                    token_symbol,
                    recipient,
                    amount,
                    price_in_sol,
                )
                .await?;

            // Handle result
            if result.success {
                self.view
                    .display_transaction_success(
                        token_symbol,
                        recipient,
                        amount,
                        result.signature.as_deref().unwrap_or("unknown"),
                        processing_message,
                    )
                    .await?;
            } else {
                self.view
                    .display_transaction_error(
                        token_symbol,
                        recipient,
                        amount,
                        result
                            .error_message
                            .unwrap_or_else(|| "Unknown error".to_string()),
                        processing_message,
                    )
                    .await?;
            }
        } else {
            // User cancelled the transaction
            self.view.display_transaction_cancelled().await?;
        }

        Ok(())
    }
}
