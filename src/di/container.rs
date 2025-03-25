use std::sync::Arc;

use solana_client::nonblocking::rpc_client::RpcClient;
use sqlx::PgPool;

use crate::solana::jupiter::config::Config as JupiterConfig;
use crate::solana::jupiter::price_service::JupiterPriceService;
use crate::solana::jupiter::price_service::PriceService;
use crate::solana::jupiter::quote_service::JupiterQuoteService;
use crate::solana::jupiter::quote_service::QuoteService;
use crate::solana::jupiter::route_service::JupiterRouteService;
use crate::solana::jupiter::route_service::RouteService;
use crate::solana::jupiter::swap_service::SwapService;
use crate::solana::jupiter::token_repository::JupiterTokenRepository;
use crate::solana::jupiter::token_repository::TokenRepository;

/// ServiceContainer provides access to core application dependencies
pub struct ServiceContainer {
    // Core services
    db_pool: Arc<PgPool>,
    solana_client: Arc<RpcClient>,

    // Jupiter services
    token_repository: Arc<dyn TokenRepository + Send + Sync>,
    quote_service: Arc<dyn QuoteService + Send + Sync>,
    price_service: Arc<dyn PriceService + Send + Sync>,
    route_service: Arc<dyn RouteService + Send + Sync>,

    // We need to use concrete types for SwapService as it requires concrete types in its generic parameters
    swap_service:
        Arc<SwapService<JupiterTokenRepository, JupiterQuoteService<JupiterTokenRepository>>>,

    // Configuration
    jupiter_config: JupiterConfig,
}

impl ServiceContainer {
    /// Create a new service container with essential dependencies
    pub fn new(db_pool: Arc<PgPool>, solana_client: Arc<RpcClient>) -> Self {
        let db_pool = db_pool;
        let solana_client = solana_client;

        // Create configuration
        let jupiter_config = JupiterConfig::from_env();

        // Initialize repositories
        let token_repository =
            Arc::new(JupiterTokenRepository::new()) as Arc<dyn TokenRepository + Send + Sync>;

        // Initialize services
        let quote_service = Arc::new(JupiterQuoteService::new(JupiterTokenRepository::new()))
            as Arc<dyn QuoteService + Send + Sync>;

        // Create a price service
        let price_service = Arc::new(JupiterPriceService::new(
            JupiterTokenRepository::new(),
            JupiterQuoteService::new(JupiterTokenRepository::new()),
            jupiter_config.clone(),
        )) as Arc<dyn PriceService + Send + Sync>;

        // Create a route service
        let route_service = Arc::new(JupiterRouteService::new(jupiter_config.clone()))
            as Arc<dyn RouteService + Send + Sync>;

        // Create swap service with concrete types
        let swap_service = Arc::new(SwapService::new(
            JupiterTokenRepository::new(),
            JupiterQuoteService::new(JupiterTokenRepository::new()),
        ));

        Self {
            db_pool,
            solana_client,
            token_repository,
            quote_service,
            price_service,
            route_service,
            swap_service,
            jupiter_config,
        }
    }

    // Accessor methods

    pub fn db_pool(&self) -> Arc<PgPool> {
        self.db_pool.clone()
    }

    pub fn solana_client(&self) -> Arc<RpcClient> {
        self.solana_client.clone()
    }

    pub fn token_repository(&self) -> Arc<dyn TokenRepository + Send + Sync> {
        self.token_repository.clone()
    }

    pub fn quote_service(&self) -> Arc<dyn QuoteService + Send + Sync> {
        self.quote_service.clone()
    }

    pub fn price_service(&self) -> Arc<dyn PriceService + Send + Sync> {
        self.price_service.clone()
    }

    pub fn route_service(&self) -> Arc<dyn RouteService + Send + Sync> {
        self.route_service.clone()
    }

    pub fn swap_service(
        &self,
    ) -> Arc<SwapService<JupiterTokenRepository, JupiterQuoteService<JupiterTokenRepository>>> {
        self.swap_service.clone()
    }

    pub fn jupiter_config(&self) -> JupiterConfig {
        self.jupiter_config.clone()
    }
}
