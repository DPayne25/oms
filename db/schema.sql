-- traders
CREATE TABLE IF NOT EXISTS traders (
    trader_id SERIAL PRIMARY KEY,
    username VARCHAR(255) UNIQUE NOT NULL,
    first_name VARCHAR(255) NOT NULL,
    last_name VARCHAR(255) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- enums
DO $$ 
BEGIN

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'account_role') THEN
        CREATE TYPE account_role AS ENUM ('master', 'slave');
    END IF;


    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'capital_source') THEN
        CREATE TYPE capital_source AS ENUM ('prop', 'personal', 'cta');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'drawdown_type') THEN
        CREATE TYPE drawdown_type AS ENUM ('trailing', 'fixed');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'side') THEN
        CREATE TYPE side AS ENUM ('buy', 'sell');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'trade_status') THEN
        CREATE TYPE trade_status AS ENUM ('pending', 'open', 'partially_closed', 'closed', 'cancelled');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'data_source') THEN
        CREATE TYPE data_source AS ENUM ('live', 'imported', 'manual');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'stops_reason') THEN
        CREATE TYPE stops_reason AS ENUM ('level_break', 'atr_trail', 'manual');
    END IF;

END 
$$;


-- accounts
CREATE TABLE IF NOT EXISTS accounts (
    account_id SERIAL PRIMARY KEY,
    currency VARCHAR(3) NOT NULL DEFAULT 'USD',
    account_number VARCHAR(255) UNIQUE NOT NULL,
    broker_name VARCHAR(255) NOT NULL,
    platform_name VARCHAR(255) NOT NULL,
    leverage INTEGER NOT NULL,
    trader_id INTEGER NOT NULL REFERENCES traders(trader_id),
    role account_role NOT NULL,
    capital_source capital_source NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP    
);

-- trades
CREATE TABLE IF NOT EXISTS trades (
    trade_id SERIAL PRIMARY KEY,
    account_id INTEGER NOT NULL REFERENCES accounts(account_id),
    symbol VARCHAR(30) NOT NULL,
    side trade_type NOT NULL,
    setup VARCHAR(25) DEFAULT NULL,
    lot_size NUMERIC(5, 2) NOT NULL,
    open_time TIMESTAMPTZ NOT NULL,
    open_price NUMERIC(10, 5) NOT NULL,
    initial_stop_loss NUMERIC(10, 5) DEFAULT NULL,
    initial_take_profit NUMERIC(10, 5) DEFAULT NULL,
    close_time TIMESTAMPTZ DEFAULT NULL,
    close_price NUMERIC(10, 5) DEFAULT NULL,
    sl_was_modified BOOLEAN DEFAULT FALSE,
    tp_was_modified BOOLEAN DEFAULT FALSE,
    commission NUMERIC DEFAULT NULL,
    swap NUMERIC DEFAULT NULL,
    gross_profit NUMERIC DEFAULT NULL,
    net_profit NUMERIC DEFAULT NULL,
    status trade_status NOT NULL,
    source data_source NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- stops
CREATE TABLE IF NOT EXISTS stops (
    stop_id SERIAL PRIMARY KEY,
    trade_id INTEGER NOT NULL REFERENCES trades(trade_id),
    adjustment_time TIMESTAMPTZ NOT NULL,
    reason stops_reason NOT NULL,
    stop_price NUMERIC(10, 5) NOT NULL,
    prv_stop_price NUMERIC(10, 5) DEFAULT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- staging_orders
CREATE TABLE IF NOT EXISTS staging_orders (
    staging_order_id SERIAL PRIMARY KEY,
    account_id INTEGER NOT NULL REFERENCES accounts(account_id),
    raw_payload JSONB NOT NULL,
    tradelocker_order_id TEXT GENERATED ALWAYS AS (raw_payload->>'id') STORED,
    fetched_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    processed_into_trade_id INTEGER DEFAULT NULL REFERENCES trades(trade_id),
    UNIQUE (account_id, tradelocker_order_id)
);