// src/commands/price.rs
use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use sqlx::PgPool;
use std::sync::Arc;
use teloxide::prelude::*;

use super::CommandHandler;
use crate::solana::jupiter::price_service::JupiterPriceService;
use crate::solana::jupiter::quote_service::JupiterQuoteService;
use crate::solana::jupiter::route_service::JupiterRouteService;
use crate::solana::jupiter::token_repository::JupiterTokenRepository;
use crate::solana::jupiter::{Config, PriceService, RouteService};
use crate::MyDialogue;

pub struct PriceCommand;

impl CommandHandler for PriceCommand {
    fn command_name() -> &'static str {
        "price"
    }

    fn description() -> &'static str {
        "get price for a token"
    }

    async fn execute(
        bot: Bot,
        msg: Message,
        _dialogue: Option<MyDialogue>,
        _db_pool: Option<PgPool>,
        _solana_client: Option<Arc<RpcClient>>,
    ) -> Result<()> {
        let command_parts: Vec<&str> = msg.text().unwrap_or("").split_whitespace().collect();

        if command_parts.len() >= 2 {
            let token = command_parts[1];

            let processing_msg = bot
                .send_message(msg.chat.id, format!("Получение цены для {}...", token))
                .await?;

            let mut route_service = JupiterRouteService::new(Config::from_env());
            let price_service = JupiterPriceService::new(
                JupiterTokenRepository::new(),
                JupiterQuoteService::new(JupiterTokenRepository::new()),
                Config::from_env(),
            );
            match price_service.get_token_price(&token).await {
                Ok(price_info) => {
                    // price_info — это структура TokenPrice
                    // Чтобы вывести её в текст, обращаемся к нужным полям,
                    // например price_in_usdc или price_in_sol
                    bot.edit_message_text(
                        msg.chat.id,
                        processing_msg.id,
                        format!(
                            "Текущая цена {}:\n≈ {:.6} SOL\n≈ {:.6} USDC",
                            token, price_info.price_in_sol, price_info.price_in_usdc,
                        ),
                    )
                    .await?;
                }
                Err(e) => {
                    bot.edit_message_text(
                        msg.chat.id,
                        processing_msg.id,
                        format!("❌ Ошибка при получении цены: {}", e),
                    )
                    .await?;
                }
            }
        } else {
            bot.send_message(
                msg.chat.id,
                "Используйте команду в формате: /price <символ_токена>\n\nПример: /price SOL",
            )
            .await?;
        }

        Ok(())
    }
}
