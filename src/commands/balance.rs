use anyhow::Result;
use log::{error, info};
use solana_client::nonblocking::rpc_client::RpcClient;
use sqlx::PgPool;
use std::sync::Arc;
use teloxide::{prelude::*, types::ParseMode};

use super::CommandHandler;
use crate::di::ServiceContainer;
use crate::model::TokenBalance;
use crate::MyDialogue;
use crate::{db, solana};

pub struct BalanceCommand;

impl CommandHandler for BalanceCommand {
    fn command_name() -> &'static str {
        "balance"
    }

    fn description() -> &'static str {
        "check your wallet balance and token holdings"
    }

    async fn execute(
        bot: Bot,
        msg: Message,
        _dialogue: Option<MyDialogue>,
        solana_client: Option<Arc<RpcClient>>,
        services: Arc<ServiceContainer>,
    ) -> Result<()> {
        let db_pool = &services.db_pool();
        let solana_client = &services.solana_client();
        let price_service = &services.price_service();

        let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);

        info!("Balance command received from Telegram ID: {}", telegram_id);

        // Get user's wallet address
        let user = db::get_user_by_telegram_id(&db_pool, telegram_id).await?;

        if let Some(address) = user.solana_address {
            // Send a status message
            let status_message = bot
                .send_message(msg.chat.id, "Fetching balance and token information...")
                .await?;

            // Get SOL balance
            let sol_balance = solana::get_sol_balance(&solana_client, &address).await?;

            // Get token balances
            let token_balances = match solana::get_token_balances(&solana_client, &address).await {
                Ok(balances) => balances,
                Err(e) => {
                    error!("Error fetching token balances: {:?}", e);
                    vec![] // Empty vector if error
                }
            };

            // Initialize vector for USD values
            let mut usd_values = Vec::new();

            if !token_balances.is_empty() {
                // Get SOL price first for reference
                let sol_price = match price_service.get_sol_price().await {
                    Ok(price) => price,
                    Err(e) => {
                        error!("Error fetching SOL price: {:?}", e);
                        0.0 // Default to 0 if error
                    }
                };

                // Calculate SOL USD value
                let sol_usd = sol_balance * sol_price;
                usd_values.push((String::from("SOL"), sol_usd));

                // Get prices for other tokens
                for token in &token_balances {
                    if token.amount > 0.0 {
                        match price_service.get_token_price(&token.mint_address).await {
                            Ok(price_info) => {
                                let usd_value = token.amount * price_info.price_in_usdc;
                                usd_values.push((token.symbol.clone(), usd_value));
                            }
                            Err(e) => {
                                error!("Error fetching price for {}: {:?}", token.symbol, e);
                                usd_values.push((token.symbol.clone(), 0.0)); // Default to 0 if error
                            }
                        }
                    }
                }
            }

            // Calculate total USD value
            let total_usd: f64 = usd_values.iter().map(|(_, value)| value).sum();

            // Format balances with USD values if available
            let mut response = format!("💰 **Wallet Balance {}**\n\n", format_address(&address));

            // Show SOL balance with USD
            let sol_usd = usd_values
                .iter()
                .find(|(symbol, _)| symbol == "SOL")
                .map(|(_, value)| *value)
                .unwrap_or(0.0);
            response.push_str(&format!(
                "• **SOL**: {:.6} (~${:.2})\n",
                sol_balance, sol_usd
            ));

            // Sort tokens by USD value (descending)
            let mut token_display: Vec<(TokenBalance, f64)> = token_balances
                .iter()
                .map(|token| {
                    let usd = usd_values
                        .iter()
                        .find(|(sym, _)| sym == &token.symbol)
                        .map(|(_, val)| *val)
                        .unwrap_or(0.0);
                    (token.clone(), usd)
                })
                .filter(|(token, _)| token.amount > 0.0) // Filter out zero balances
                .collect();

            token_display.sort_by(|(_, usd1), (_, usd2)| {
                usd2.partial_cmp(usd1).unwrap_or(std::cmp::Ordering::Equal)
            });

            // Add token balances with USD values
            if !token_display.is_empty() {
                response.push_str("\n**SPL Tokens:**\n");
                for (token, usd) in token_display {
                    if usd > 0.0 {
                        response.push_str(&format!(
                            "• **{}**: {:.6} (~${:.2})\n",
                            token.symbol, token.amount, usd
                        ));
                    } else {
                        response
                            .push_str(&format!("• **{}**: {:.6}\n", token.symbol, token.amount));
                    }
                }
            }

            // Add total portfolio value
            if total_usd > 0.0 {
                response.push_str(&format!("\n**Total Portfolio Value:** ~${:.2}", total_usd));
            }

            // Update the status message with the balance info
            bot.edit_message_text(msg.chat.id, status_message.id, response)
                .parse_mode(ParseMode::Markdown)
                .await?;
        } else {
            bot.send_message(
                msg.chat.id,
                "You don't have a wallet yet. Use /create_wallet to create a new wallet.",
            )
            .await?;
        }

        Ok(())
    }
}

// Helper function to format wallet address
fn format_address(address: &str) -> String {
    if address.len() <= 12 {
        return address.to_string();
    }
    format!("{}...{}", &address[..6], &address[address.len() - 4..])
}
