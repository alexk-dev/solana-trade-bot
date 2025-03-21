use crate::entity::{Swap, Transaction, User};
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
