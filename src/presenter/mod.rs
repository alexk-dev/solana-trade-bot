use async_trait::async_trait;
use teloxide::prelude::*;

pub mod balance_presenter;
pub mod price_presenter;
pub mod send_presenter;
pub mod swap_presenter;
pub mod wallet_presenter;

// Base presenter trait
#[async_trait]
pub trait Presenter: Send + Sync {
    // Each presenter implementation will define its specific methods
}
