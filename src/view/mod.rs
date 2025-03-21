use async_trait::async_trait;
use teloxide::prelude::*;

pub mod balance_view;
pub mod price_view;
pub mod send_view;
pub mod swap_view;
pub mod wallet_view;

// Base view trait
#[async_trait]
pub trait View: Send + Sync {
    // Each view implementation will define its specific methods
}
