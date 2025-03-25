use crate::interactor::settings_interactor::SettingsInteractor;
use crate::view::settings_view::SettingsView;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait SettingsPresenter: Send + Sync {
    async fn show_settings_menu(&self, telegram_id: i64) -> Result<()>;
    async fn show_slippage_prompt(&self, telegram_id: i64) -> Result<()>;
    async fn update_slippage(&self, telegram_id: i64, slippage_text: &str) -> Result<()>;
    async fn set_preset_slippage(&self, telegram_id: i64, slippage: f64) -> Result<()>;
}

pub struct SettingsPresenterImpl<I, V> {
    interactor: Arc<I>,
    view: Arc<V>,
}

impl<I, V> SettingsPresenterImpl<I, V>
where
    I: SettingsInteractor,
    V: SettingsView,
{
    pub fn new(interactor: Arc<I>, view: Arc<V>) -> Self {
        Self { interactor, view }
    }
}

#[async_trait]
impl<I, V> SettingsPresenter for SettingsPresenterImpl<I, V>
where
    I: SettingsInteractor + Send + Sync,
    V: SettingsView + Send + Sync,
{
    async fn show_settings_menu(&self, telegram_id: i64) -> Result<()> {
        // Get user settings
        match self.interactor.get_user_settings(telegram_id).await {
            Ok(user) => {
                let slippage = user.get_slippage();
                self.view.display_settings_menu(slippage).await?;
            }
            Err(e) => {
                self.view.display_error(e.to_string()).await?;
            }
        }

        Ok(())
    }

    async fn show_slippage_prompt(&self, telegram_id: i64) -> Result<()> {
        // Get current slippage value
        match self.interactor.get_user_settings(telegram_id).await {
            Ok(user) => {
                let current_slippage = user.get_slippage();
                self.view.display_slippage_prompt(current_slippage).await?;
            }
            Err(e) => {
                self.view.display_error(e.to_string()).await?;
            }
        }

        Ok(())
    }

    async fn update_slippage(&self, telegram_id: i64, slippage_text: &str) -> Result<()> {
        // Parse slippage percentage
        match slippage_text.trim().trim_end_matches('%').parse::<f64>() {
            Ok(slippage) => {
                // Update slippage in database
                match self.interactor.update_slippage(telegram_id, slippage).await {
                    Ok(updated_slippage) => {
                        self.view.display_slippage_updated(updated_slippage).await?;
                    }
                    Err(e) => {
                        self.view.display_error(e.to_string()).await?;
                    }
                }
            }
            Err(_) => {
                self.view
                    .display_invalid_slippage("Invalid number format".to_string())
                    .await?;
            }
        }

        Ok(())
    }

    async fn set_preset_slippage(&self, telegram_id: i64, slippage: f64) -> Result<()> {
        // Update slippage in database
        match self.interactor.update_slippage(telegram_id, slippage).await {
            Ok(updated_slippage) => {
                self.view.display_slippage_updated(updated_slippage).await?;
            }
            Err(e) => {
                self.view.display_error(e.to_string()).await?;
            }
        }

        Ok(())
    }
}
