use anyhow::Result;
use log;
use std::sync::Arc;
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
};

use super::{CommandHandler, MyDialogue};
use crate::di::ServiceContainer;
use crate::entity::State;

pub struct SwapCommand;

impl CommandHandler for SwapCommand {
    fn command_name() -> &'static str {
        "swap"
    }

    fn description() -> &'static str {
        "swap tokens via Jupiter DEX"
    }

    async fn execute(
        bot: Bot,
        msg: Message,
        telegram_id: i64,
        dialogue: Option<MyDialogue>,
        services: Arc<ServiceContainer>,
    ) -> Result<()> {
        let chat_id = msg.chat.id;

        // Instead of parsing command parts, show token selection keyboard
        let source_token_keyboard = InlineKeyboardMarkup::new(vec![
            vec![
                InlineKeyboardButton::callback("SOL", "swap_from_SOL"),
                InlineKeyboardButton::callback("USDC", "swap_from_USDC"),
                InlineKeyboardButton::callback("USDT", "swap_from_USDT"),
            ],
            vec![
                InlineKeyboardButton::callback("RAY", "swap_from_RAY"),
                InlineKeyboardButton::callback("← Back", "main_menu"),
            ],
        ]);

        bot.send_message(chat_id, "Select source token to swap from:")
            .reply_markup(source_token_keyboard)
            .await?;

        // The actual swap will be handled by callback handlers

        Ok(())
    }
}

// Handle receiving swap details in chat (for when a user is in the swap flow)
pub async fn receive_swap_details(bot: Bot, msg: Message, dialogue: MyDialogue) -> Result<()> {
    dialogue.update(State::Start).await?;

    // Create buttons for swapping specific tokens
    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("SOL → USDC", "swap_SOL_USDC"),
            InlineKeyboardButton::callback("USDC → SOL", "swap_USDC_SOL"),
        ],
        vec![
            InlineKeyboardButton::callback("SOL → USDT", "swap_SOL_USDT"),
            InlineKeyboardButton::callback("USDT → SOL", "swap_USDT_SOL"),
        ],
        vec![InlineKeyboardButton::callback(
            "← Back to Menu",
            "main_menu",
        )],
    ]);

    bot.send_message(
        msg.chat.id,
        "Choose a token pair to swap or use the format: /swap amount from_token to_token",
    )
    .reply_markup(keyboard)
    .await?;

    Ok(())
}

// This function handles the second step - selecting target token
pub async fn handle_swap_from_selection(
    bot: &Bot,
    chat_id: ChatId,
    source_token: &str,
) -> Result<()> {
    // Show target token selection
    let target_token_keyboard = InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback(
                format!("Swap {} → SOL", source_token),
                format!("swap_{}_SOL", source_token),
            ),
            InlineKeyboardButton::callback(
                format!("Swap {} → USDC", source_token),
                format!("swap_{}_USDC", source_token),
            ),
        ],
        vec![
            InlineKeyboardButton::callback(
                format!("Swap {} → USDT", source_token),
                format!("swap_{}_USDT", source_token),
            ),
            InlineKeyboardButton::callback(
                format!("Swap {} → RAY", source_token),
                format!("swap_{}_RAY", source_token),
            ),
        ],
        vec![InlineKeyboardButton::callback("← Back", "swap")],
    ]);

    // Exclude the source token from target options
    let filtered_keyboard = filter_token_keyboard(target_token_keyboard, source_token);

    bot.send_message(
        chat_id,
        format!("Select token to swap {} to:", source_token),
    )
    .reply_markup(filtered_keyboard)
    .await?;

    Ok(())
}

// This function handles the third step - entering amount
pub async fn handle_swap_pair_selection(
    bot: &Bot,
    chat_id: ChatId,
    source_token: &str,
    target_token: &str,
) -> Result<()> {
    // Show amount options
    let amount_keyboard = InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback(
                format!("Swap 0.1 {}", source_token),
                format!("swap_amount_0.1_{}_to_{}", source_token, target_token),
            ),
            InlineKeyboardButton::callback(
                format!("Swap 0.5 {}", source_token),
                format!("swap_amount_0.5_{}_to_{}", source_token, target_token),
            ),
        ],
        vec![
            InlineKeyboardButton::callback(
                format!("Swap 1 {}", source_token),
                format!("swap_amount_1_{}_to_{}", source_token, target_token),
            ),
            InlineKeyboardButton::callback(
                format!("Swap 5 {}", source_token),
                format!("swap_amount_5_{}_to_{}", source_token, target_token),
            ),
        ],
        vec![InlineKeyboardButton::callback(
            "Custom Amount",
            format!("swap_custom_{}_{}", source_token, target_token),
        )],
        vec![InlineKeyboardButton::callback(
            "← Back",
            format!("swap_from_{}", source_token),
        )],
    ]);

    bot.send_message(
        chat_id,
        format!(
            "Select amount of {} to swap to {}:",
            source_token, target_token
        ),
    )
    .reply_markup(amount_keyboard)
    .await?;

    Ok(())
}

// Helper function to filter out source token from target options
fn filter_token_keyboard(
    keyboard: InlineKeyboardMarkup,
    source_token: &str,
) -> InlineKeyboardMarkup {
    let filtered_rows = keyboard
        .inline_keyboard
        .into_iter()
        .map(|row| {
            row.into_iter()
                .filter(|button| {
                    if let teloxide::types::InlineKeyboardButtonKind::CallbackData(callback) =
                        &button.kind
                    {
                        // Filter out buttons that would swap to the same token
                        !callback.contains(&format!("swap_{0}_{0}", source_token))
                    } else {
                        true
                    }
                })
                .collect::<Vec<_>>()
        })
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();

    InlineKeyboardMarkup::new(filtered_rows)
}
