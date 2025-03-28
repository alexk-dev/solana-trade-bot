use async_trait::async_trait;

pub mod balance_view;
pub mod limit_order_view;
pub mod price_view;
pub mod send_view;
pub mod settings_view;
pub mod trade_view;
pub mod wallet_view;
pub(crate) mod watchlist_view;
pub(crate) mod withdraw_view;

// Base view trait
#[async_trait]
pub trait View: Send + Sync {
    // Each view implementation will define its specific methods
}
