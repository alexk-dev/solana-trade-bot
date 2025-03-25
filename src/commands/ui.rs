use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn create_wallet_menu_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("Buy", "buy"),
            InlineKeyboardButton::callback("Sell", "sell"),
            InlineKeyboardButton::callback("Watchlist", "watchlist"),
        ],
        vec![
            InlineKeyboardButton::callback("Check Price", "price"),
            InlineKeyboardButton::callback("Limit Orders", "limit_orders"),
        ],
        vec![
            InlineKeyboardButton::callback("Withdraw", "send"),
            InlineKeyboardButton::callback("View Address", "address"),
        ],
        vec![
            InlineKeyboardButton::callback("Help", "help"),
            InlineKeyboardButton::callback("Settings", "settings"),
            InlineKeyboardButton::callback("🔄 Refresh", "refresh"),
        ],
    ])
}
