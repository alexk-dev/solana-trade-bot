use async_trait::async_trait;

pub mod balance_presenter;
pub mod limit_order_presenter;
pub mod price_presenter;
pub mod send_presenter;
pub mod settings_presenter;
pub mod trade_presenter;
pub mod wallet_presenter;
pub mod watchlist_presenter;
pub(crate) mod withdraw_presenter;

// Base presenter trait
#[async_trait]
pub trait Presenter: Send + Sync {
    // Each presenter implementation will define its specific methods
}
