use crate::entity::{LimitOrder, LimitOrderStatus, OrderType, Swap, Trade, Transaction, User};
use chrono::Utc;
use log::info;
use sqlx::{postgres::PgQueryResult, Error as SqlxError, PgPool, Row};

// Check if user exists in database
pub async fn check_user_exists(pool: &PgPool, telegram_id: i64) -> Result<bool, SqlxError> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM users WHERE telegram_id = $1")
        .bind(telegram_id)
        .fetch_one(pool)
        .await?;

    let count: i64 = row.try_get("count")?;
    Ok(count > 0)
}

// Create new user in database
pub async fn create_user(
    pool: &PgPool,
    telegram_id: i64,
    username: Option<String>,
) -> Result<i32, SqlxError> {
    let row = sqlx::query(
        "INSERT INTO users (telegram_id, username, created_at) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(telegram_id)
    .bind(username)
    .bind(Utc::now())
    .fetch_one(pool)
    .await?;

    let id: i32 = row.try_get("id")?;
    info!("Created new user with ID: {}", id);

    Ok(id)
}

// Get user by telegram_id
pub async fn get_user_by_telegram_id(pool: &PgPool, telegram_id: i64) -> Result<User, SqlxError> {
    let row = sqlx::query("SELECT * FROM users WHERE telegram_id = $1")
        .bind(telegram_id)
        .fetch_one(pool)
        .await?;

    let user = User {
        id: row.try_get("id")?,
        telegram_id: row.try_get("telegram_id")?,
        username: row.try_get("username")?,
        solana_address: row.try_get("solana_address")?,
        encrypted_private_key: row.try_get("encrypted_private_key")?,
        mnemonic: row.try_get("mnemonic")?,
        settings: row.try_get("settings")?,
        created_at: row.try_get("created_at")?,
    };

    Ok(user)
}

// Save wallet information for a user
pub async fn save_wallet_info(
    pool: &PgPool,
    telegram_id: i64,
    address: &str,
    keypair: &str,
    mnemonic: &str,
) -> Result<PgQueryResult, SqlxError> {
    let result = sqlx::query("UPDATE users SET solana_address = $1, encrypted_private_key = $2, mnemonic = $3 WHERE telegram_id = $4")
        .bind(address)
        .bind(keypair)
        .bind(mnemonic)
        .bind(telegram_id)
        .execute(pool)
        .await?;

    info!(
        "Updated wallet info for user with Telegram ID: {}",
        telegram_id
    );

    Ok(result)
}

// Record a transaction in the database
pub async fn record_transaction(
    pool: &PgPool,
    telegram_id: i64,
    recipient_address: &str,
    amount: f64,
    token_symbol: &str,
    tx_signature: &Option<String>,
    status: &str,
) -> Result<i32, SqlxError> {
    // Get user ID from telegram_id
    let user = get_user_by_telegram_id(pool, telegram_id).await?;

    let row = sqlx::query("INSERT INTO transactions (user_id, recipient_address, amount, token_symbol, tx_signature, timestamp, status) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id")
        .bind(user.id)
        .bind(recipient_address)
        .bind(amount)
        .bind(token_symbol)
        .bind(tx_signature.as_deref())
        .bind(Utc::now())
        .bind(status)
        .fetch_one(pool)
        .await?;

    let id: i32 = row.try_get("id")?;
    info!("Recorded transaction with ID: {}", id);

    Ok(id)
}

// Record a swap operation in the database
pub async fn record_swap(
    pool: &PgPool,
    telegram_id: i64,
    from_token: &str,
    to_token: &str,
    amount_in: f64,
    amount_out: f64,
    tx_signature: &Option<String>,
    status: &str,
) -> Result<i32, SqlxError> {
    // Get user ID from telegram_id
    let user = get_user_by_telegram_id(pool, telegram_id).await?;

    let row = sqlx::query("INSERT INTO swaps (user_id, from_token, to_token, amount_in, amount_out, tx_signature, timestamp, status) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id")
        .bind(user.id)
        .bind(from_token)
        .bind(to_token)
        .bind(amount_in)
        .bind(amount_out)
        .bind(tx_signature.as_deref())
        .bind(Utc::now())
        .bind(status)
        .fetch_one(pool)
        .await?;

    let id: i32 = row.try_get("id")?;
    info!("Recorded swap with ID: {}", id);

    Ok(id)
}

// Get user transaction history
pub async fn get_user_transactions(
    pool: &PgPool,
    telegram_id: i64,
) -> Result<Vec<Transaction>, SqlxError> {
    // Get user ID from telegram_id
    let user = get_user_by_telegram_id(pool, telegram_id).await?;

    let rows = sqlx::query("SELECT * FROM transactions WHERE user_id = $1 ORDER BY timestamp DESC")
        .bind(user.id)
        .fetch_all(pool)
        .await?;

    let mut transactions = Vec::new();
    for row in rows {
        let transaction = Transaction {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            recipient_address: row.try_get("recipient_address")?,
            amount: row.try_get("amount")?,
            token_symbol: row.try_get("token_symbol")?,
            tx_signature: row.try_get("tx_signature")?,
            timestamp: row.try_get("timestamp")?,
            status: row.try_get("status")?,
        };
        transactions.push(transaction);
    }

    Ok(transactions)
}

// Get user swap history
pub async fn get_user_swaps(pool: &PgPool, telegram_id: i64) -> Result<Vec<Swap>, SqlxError> {
    // Get user ID from telegram_id
    let user = get_user_by_telegram_id(pool, telegram_id).await?;

    let rows = sqlx::query("SELECT * FROM swaps WHERE user_id = $1 ORDER BY timestamp DESC")
        .bind(user.id)
        .fetch_all(pool)
        .await?;

    let mut swaps = Vec::new();
    for row in rows {
        let swap = Swap {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            from_token: row.try_get("from_token")?,
            to_token: row.try_get("to_token")?,
            amount_in: row.try_get("amount_in")?,
            amount_out: row.try_get("amount_out")?,
            tx_signature: row.try_get("tx_signature")?,
            timestamp: row.try_get("timestamp")?,
            status: row.try_get("status")?,
        };
        swaps.push(swap);
    }

    Ok(swaps)
}

// Record a trade operation in the database
pub async fn record_trade(
    pool: &PgPool,
    telegram_id: i64,
    token_address: &str,
    token_symbol: &str,
    amount: f64,
    price_in_sol: f64,
    total_paid: f64,
    trade_type: &str,
    tx_signature: &Option<String>,
    status: &str,
) -> Result<i32, SqlxError> {
    // Get user ID from telegram_id
    let user = get_user_by_telegram_id(pool, telegram_id).await?;

    let price_in_usdc = 0.0; // In a real implementation, get the actual USDC price

    let row = sqlx::query(
        "INSERT INTO trades (user_id, token_address, token_symbol, amount, price_in_sol, price_in_usdc, total_paid, trade_type, tx_signature, timestamp, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
         RETURNING id",
    )
        .bind(user.id)
        .bind(token_address)
        .bind(token_symbol)
        .bind(amount)
        .bind(price_in_sol)
        .bind(price_in_usdc)
        .bind(total_paid)
        .bind(trade_type)
        .bind(tx_signature.as_deref())
        .bind(Utc::now())
        .bind(status)
        .fetch_one(pool)
        .await?;

    let id: i32 = row.try_get("id")?;
    info!("Recorded trade with ID: {}", id);

    Ok(id)
}

// Get user trade history
pub async fn get_user_trades(pool: &PgPool, telegram_id: i64) -> Result<Vec<Trade>, SqlxError> {
    // Get user ID from telegram_id
    let user = get_user_by_telegram_id(pool, telegram_id).await?;

    let rows = sqlx::query("SELECT * FROM trades WHERE user_id = $1 ORDER BY timestamp DESC")
        .bind(user.id)
        .fetch_all(pool)
        .await?;

    let mut trades = Vec::new();
    for row in rows {
        let trade = Trade {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            token_address: row.try_get("token_address")?,
            token_symbol: row.try_get("token_symbol")?,
            amount: row.try_get("amount")?,
            price_in_sol: row.try_get("price_in_sol")?,
            price_in_usdc: row.try_get("price_in_usdc")?,
            total_paid: row.try_get("total_paid")?,
            trade_type: row.try_get("trade_type")?,
            tx_signature: row.try_get("tx_signature")?,
            timestamp: row.try_get("timestamp")?,
            status: row.try_get("status")?,
        };
        trades.push(trade);
    }

    Ok(trades)
}

pub async fn create_limit_order(
    pool: &PgPool,
    telegram_id: i64,
    token_address: &str,
    token_symbol: &str,
    order_type: &OrderType,
    price_in_sol: f64,
    total_sol: f64,
    current_price_in_sol: Option<f64>,
) -> Result<i32, SqlxError> {
    // Get user ID from telegram_id
    let user = get_user_by_telegram_id(pool, telegram_id).await?;

    let order_type_str = order_type.to_string();
    let status = LimitOrderStatus::Active.to_string();
    let now = Utc::now();

    // Calculate token amount based on total_sol and price_in_sol
    let amount = if price_in_sol > 0.0 {
        total_sol / price_in_sol
    } else {
        0.0
    };

    let row = sqlx::query(
        "INSERT INTO limit_orders (
            user_id, token_address, token_symbol, order_type,
            price_in_sol, amount, total_sol, current_price_in_sol,
            created_at, updated_at, status, retry_count
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING id",
    )
    .bind(user.id)
    .bind(token_address)
    .bind(token_symbol)
    .bind(order_type_str)
    .bind(price_in_sol)
    .bind(amount)
    .bind(total_sol)
    .bind(current_price_in_sol)
    .bind(now)
    .bind(now)
    .bind(status)
    .bind(0) // Initial retry_count = 0
    .fetch_one(pool)
    .await?;

    let id: i32 = row.try_get("id")?;
    info!("Created new limit order with ID: {}", id);

    Ok(id)
}
/// Get user's active limit orders
pub async fn get_active_limit_orders(
    pool: &PgPool,
    telegram_id: i64,
) -> Result<Vec<LimitOrder>, SqlxError> {
    // Get user ID from telegram_id
    let user = get_user_by_telegram_id(pool, telegram_id).await?;

    let rows = sqlx::query_as::<_, LimitOrder>(
        "SELECT * FROM limit_orders
         WHERE user_id = $1 AND status = $2
         ORDER BY created_at DESC",
    )
    .bind(user.id)
    .bind(LimitOrderStatus::Active.to_string())
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Get all user's limit orders (with optional status filter)
pub async fn get_user_limit_orders(
    pool: &PgPool,
    telegram_id: i64,
    status: Option<&LimitOrderStatus>,
) -> Result<Vec<LimitOrder>, SqlxError> {
    // Get user ID from telegram_id
    let user = get_user_by_telegram_id(pool, telegram_id).await?;

    let rows = if let Some(status) = status {
        sqlx::query_as::<_, LimitOrder>(
            "SELECT * FROM limit_orders
             WHERE user_id = $1 AND status = $2
             ORDER BY updated_at DESC",
        )
        .bind(user.id)
        .bind(status.to_string())
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, LimitOrder>(
            "SELECT * FROM limit_orders
             WHERE user_id = $1
             ORDER BY updated_at DESC",
        )
        .bind(user.id)
        .fetch_all(pool)
        .await?
    };

    Ok(rows)
}

/// Update limit order status
pub async fn update_limit_order_status(
    pool: &PgPool,
    order_id: i32,
    status: &LimitOrderStatus,
    tx_signature: Option<&str>,
) -> Result<PgQueryResult, SqlxError> {
    let now = Utc::now();
    let status_str = status.to_string();

    let result = if let Some(signature) = tx_signature {
        sqlx::query(
            "UPDATE limit_orders
             SET status = $1, updated_at = $2, tx_signature = $3
             WHERE id = $4",
        )
        .bind(&status_str)
        .bind(now)
        .bind(signature)
        .bind(order_id)
        .execute(pool)
        .await?
    } else {
        sqlx::query(
            "UPDATE limit_orders
             SET status = $1, updated_at = $2
             WHERE id = $3",
        )
        .bind(&status_str)
        .bind(now)
        .bind(order_id)
        .execute(pool)
        .await?
    };

    info!(
        "Updated limit order status: id={}, status={}",
        order_id, &status_str
    );
    Ok(result)
}

/// Update current price for a limit order
pub async fn update_limit_order_current_price(
    pool: &PgPool,
    order_id: i32,
    current_price_in_sol: f64,
) -> Result<PgQueryResult, SqlxError> {
    let now = Utc::now();

    let result = sqlx::query(
        "UPDATE limit_orders
         SET current_price_in_sol = $1, updated_at = $2
         WHERE id = $3",
    )
    .bind(current_price_in_sol)
    .bind(now)
    .bind(order_id)
    .execute(pool)
    .await?;

    info!(
        "Updated limit order current price: id={}, price={}",
        order_id, current_price_in_sol
    );
    Ok(result)
}

/// Get a specific limit order by ID
pub async fn get_limit_order_by_id(
    pool: &PgPool,
    order_id: i32,
) -> Result<Option<LimitOrder>, SqlxError> {
    let order = sqlx::query_as::<_, LimitOrder>("SELECT * FROM limit_orders WHERE id = $1")
        .bind(order_id)
        .fetch_optional(pool)
        .await?;

    Ok(order)
}

/// Cancel a limit order
pub async fn cancel_limit_order(pool: &PgPool, order_id: i32) -> Result<PgQueryResult, SqlxError> {
    update_limit_order_status(pool, order_id, &LimitOrderStatus::Cancelled, None).await
}

/// Cancel all active limit orders for a user
pub async fn cancel_all_limit_orders(pool: &PgPool, telegram_id: i64) -> Result<i32, SqlxError> {
    // Get user ID from telegram_id
    let user = get_user_by_telegram_id(pool, telegram_id).await?;
    let now = Utc::now();
    let cancelled_status = LimitOrderStatus::Cancelled.to_string();

    // Update all active orders to cancelled
    let result = sqlx::query(
        "UPDATE limit_orders
         SET status = $1, updated_at = $2
         WHERE user_id = $3 AND status = $4",
    )
    .bind(cancelled_status)
    .bind(now)
    .bind(user.id)
    .bind(LimitOrderStatus::Active.to_string())
    .execute(pool)
    .await?;

    let count = result.rows_affected() as i32;
    info!("Cancelled {} limit orders for user ID: {}", count, user.id);

    Ok(count)
}

/// Update retry count for a limit order
pub async fn update_limit_order_retry_count(
    pool: &PgPool,
    order_id: i32,
    retry_count: i32,
) -> Result<PgQueryResult, SqlxError> {
    let now = Utc::now();

    let result = sqlx::query(
        "UPDATE limit_orders
         SET retry_count = $1, updated_at = $2
         WHERE id = $3",
    )
    .bind(retry_count)
    .bind(now)
    .bind(order_id)
    .execute(pool)
    .await?;

    info!(
        "Updated retry count for order ID {}: {}",
        order_id, retry_count
    );
    Ok(result)
}

/// Get all active limit orders across all users
pub async fn get_all_active_limit_orders(pool: &PgPool) -> Result<Vec<LimitOrder>, SqlxError> {
    let rows = sqlx::query_as::<_, LimitOrder>(
        "SELECT * FROM limit_orders
         WHERE status = $1
         ORDER BY created_at ASC",
    )
    .bind(LimitOrderStatus::Active.to_string())
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Get user by ID
pub async fn get_user_by_id(pool: &PgPool, user_id: i32) -> Result<User, SqlxError> {
    let row = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await?;

    Ok(row)
}

// Update user settings
pub async fn update_user_settings(
    pool: &PgPool,
    telegram_id: i64,
    settings: &serde_json::Value,
) -> Result<PgQueryResult, SqlxError> {
    let result = sqlx::query("UPDATE users SET settings = $1 WHERE telegram_id = $2")
        .bind(settings)
        .bind(telegram_id)
        .execute(pool)
        .await?;

    info!(
        "Updated settings for user with Telegram ID: {}",
        telegram_id
    );

    Ok(result)
}

// Update user slippage setting
pub async fn update_user_slippage(
    pool: &PgPool,
    telegram_id: i64,
    slippage: f64,
) -> Result<PgQueryResult, SqlxError> {
    // Get current user settings
    let user = get_user_by_telegram_id(pool, telegram_id).await?;

    // Create updated settings
    let mut settings = user.settings.unwrap_or_else(|| serde_json::json!({}));

    // Limit slippage to reasonable range (0.1% to 5%)
    let slippage = slippage.max(0.1).min(5.0);

    // Update the slippage value
    if let Some(obj) = settings.as_object_mut() {
        obj.insert("slippage".to_string(), serde_json::json!(slippage));
    }

    // Save to database
    let result = sqlx::query("UPDATE users SET settings = $1 WHERE telegram_id = $2")
        .bind(settings)
        .bind(telegram_id)
        .execute(pool)
        .await?;

    info!(
        "Updated slippage setting to {}% for user with Telegram ID: {}",
        slippage, telegram_id
    );

    Ok(result)
}
