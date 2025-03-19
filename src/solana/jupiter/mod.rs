// src/solana/jupiter/mod.rs
pub mod models;
pub mod swap_service;
pub mod token_repository;
pub mod token_service;

// Реэкспорт для удобства использования
pub use models::{
    JupiterToken, PrioritizationFeeLamports, PrioritizationFeeLamportsWrapper, QuoteParams,
    QuoteResponse, RoutePlan, SwapInfo, SwapMode, SwapRequest, SwapResponse, Token, TokenPrice,
    SOL_MINT, USDC_MINT,
};

pub use swap_service::SwapService;
pub use token_repository::TokenRepository;
pub use token_service::TokenService;
