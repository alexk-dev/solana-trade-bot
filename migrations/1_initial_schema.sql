-- Create users table
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    telegram_id BIGINT UNIQUE NOT NULL,
    username VARCHAR,
    solana_address VARCHAR,
    encrypted_private_key VARCHAR,
    mnemonic VARCHAR,
    settings JSONB DEFAULT '{"slippage": 0.5}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create transactions table
CREATE TABLE IF NOT EXISTS transactions (
    id SERIAL PRIMARY KEY,
    user_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
    recipient_address VARCHAR NOT NULL,
    amount NUMERIC NOT NULL,
    token_symbol VARCHAR NOT NULL,
    tx_signature VARCHAR,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    status VARCHAR NOT NULL
);

-- Create index on user_id in transactions
CREATE INDEX IF NOT EXISTS idx_transactions_user_id ON transactions(user_id);

CREATE TABLE IF NOT EXISTS trades (
                                      id SERIAL PRIMARY KEY,
                                      user_id INTEGER NOT NULL REFERENCES users(id),
    token_address TEXT NOT NULL,
    token_symbol TEXT NOT NULL,
    amount DOUBLE PRECISION NOT NULL,
    price_in_sol DOUBLE PRECISION NOT NULL,
    price_in_usdc DOUBLE PRECISION NOT NULL,
    total_paid DOUBLE PRECISION NOT NULL,
    trade_type TEXT NOT NULL, -- "BUY" or "SELL"
    tx_signature TEXT,
    timestamp TIMESTAMPTZ NOT NULL,
    status TEXT NOT NULL,

    -- Add indexes for common queries
    CONSTRAINT trade_type_check CHECK (trade_type IN ('BUY', 'SELL'))
    );

-- Add indexes
CREATE INDEX IF NOT EXISTS idx_trades_user_id ON trades(user_id);
CREATE INDEX IF NOT EXISTS idx_trades_token_address ON trades(token_address);
CREATE INDEX IF NOT EXISTS idx_trades_timestamp ON trades(timestamp);

CREATE TABLE IF NOT EXISTS limit_orders (
                                            id SERIAL PRIMARY KEY,
                                            user_id INTEGER NOT NULL REFERENCES users(id),
    token_address TEXT NOT NULL,
    token_symbol TEXT NOT NULL,
    order_type TEXT NOT NULL,
    price_in_sol DOUBLE PRECISION NOT NULL,
    amount DOUBLE PRECISION NOT NULL,
    total_sol DOUBLE PRECISION NOT NULL,
    current_price_in_sol DOUBLE PRECISION,
    tx_signature TEXT,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    status TEXT NOT NULL,
    retry_count INTEGER NOT NULL DEFAULT 0
    );

-- Create index for faster queries
CREATE INDEX IF NOT EXISTS idx_limit_orders_user_id ON limit_orders(user_id);
CREATE INDEX IF NOT EXISTS idx_limit_orders_status ON limit_orders(status);