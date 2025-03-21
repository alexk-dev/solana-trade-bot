use async_trait::async_trait;

pub mod balance_interactor;
pub mod db;
pub mod price_interactor;
pub mod send_interactor;
pub mod swap_interactor;
pub mod wallet_interactor;

// Base interactor trait
#[async_trait]
pub trait Interactor: Send + Sync {
    // Each interactor implementation will define its specific methods
}
