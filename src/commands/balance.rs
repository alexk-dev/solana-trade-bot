// src/commands/balance.rs
use anyhow::Result;
use log::info;
use sqlx::PgPool;
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use teloxide::{prelude::*, types::ParseMode};

use crate::{db, solana};
use crate::MyDialogue;
use super::CommandHandler;

pub struct BalanceCommand;

impl CommandHandler for BalanceCommand {
    fn command_name() -> &'static str {
        "balance"
    }

    fn description() -> &'static str {
        "check your wallet balance"
    }

    async fn execute(
        bot: Bot,
        msg: Message,
        _dialogue: Option<MyDialogue>,
        db_pool: Option<PgPool>,
        solana_client: Option<Arc<RpcClient>>
    ) -> Result<()> {
        let db_pool = db_pool.ok_or_else(|| anyhow::anyhow!("Database pool not provided"))?;
        let solana_client = solana_client.ok_or_else(|| anyhow::anyhow!("Solana client not provided"))?;
        let telegram_id = msg.from().map_or(0, |user| user.id.0 as i64);

        info!("Balance command received from Telegram ID: {}", telegram_id);

        // Get user's wallet address
        let user = db::get_user_by_telegram_id(&db_pool, telegram_id).await?;

        if let Some(address) = user.solana_address {
            // Send a status message
            let status_message = bot.send_message(
                msg.chat.id,
                "Получение информации о балансе..."
            ).await?;

            // Get SOL balance
            let sol_balance = solana::get_sol_balance(&solana_client, &address).await?;

            // Get token balances
            let token_balances = solana::get_token_balances(&solana_client, &address).await?;

            // Prepare response message
            let mut response = format!("Баланс вашего кошелька:\n\nSOL: {:.6}", sol_balance);

            if !token_balances.is_empty() {
                for token in token_balances {
                    response.push_str(&format!("\n{}: {:.6}", token.symbol, token.amount));
                }
            }

            // Update the status message with the balance info
            bot.edit_message_text(msg.chat.id, status_message.id, response)
                .parse_mode(ParseMode::Markdown)
                .await?;
        } else {
            bot.send_message(
                msg.chat.id,
                "У вас еще нет кошелька. Используйте /create_wallet чтобы создать новый кошелек."
            )
                .await?;
        }

        Ok(())
    }
}