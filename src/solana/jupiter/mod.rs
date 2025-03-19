// src/solana/jupiter/mod.rs
pub mod models;
pub mod token_repository;
pub mod token_service;
pub mod swap_service;

// Реэкспорт для удобства использования
pub use models::{
    Token,
    TokenPrice,
    JupiterToken,
    SwapMode,
    QuoteParams,
    QuoteResponse,
    RoutePlan,
    SwapInfo,
    PrioritizationFeeLamports,
    SwapRequest,
    PrioritizationFeeLamportsWrapper,
    SwapResponse,
    SOL_MINT,
    USDC_MINT
};

pub use token_repository::TokenRepository;
pub use token_service::TokenService;
pub use swap_service::SwapService;