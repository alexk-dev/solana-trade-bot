use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn create_wallet_menu_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("Buy", "buy"),
            InlineKeyboardButton::callback("Sell", "sell"),
        ],
        vec![
            InlineKeyboardButton::callback("Positions", "positions"),
            InlineKeyboardButton::callback("Limit Orders", "limit_orders"),
            InlineKeyboardButton::callback("Check Price", "price"),
        ],
        vec![
            InlineKeyboardButton::callback("📤 Withdraw", "send"),
            InlineKeyboardButton::callback("🔑 View Address", "address"),
            InlineKeyboardButton::callback("Settings", "settings"),
        ],
        vec![
            InlineKeyboardButton::callback("Help", "help"),
            InlineKeyboardButton::callback("🔄 Refresh", "refresh"),
        ],
    ])
}
