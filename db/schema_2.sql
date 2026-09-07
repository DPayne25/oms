-- traders
CREATE TABLE IF NOT EXISTS traders (
    id SERIAL PRIMARY KEY,
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
        CREATE TYPE account_role AS ENUM ('aggressive', 'conservative', 'test');
    END IF;


    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'capital_source') THEN
        CREATE TYPE capital_source AS ENUM ('prop', 'personal', '3_p');
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

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'order_type') THEN
        CREATE TYPE order_type AS ENUM ('limit', 'market_execution', 'stop');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'stops_reason') THEN
        CREATE TYPE stops_reason AS ENUM ('level_break', 'atr_trail', 'manual');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'order_status') THEN
        CREATE TYPE order_status AS ENUM ('New', 'Pending', 'Filled', 'Cancelled', 'Rejected', 'Expired');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'time_frame') THEN
        CREATE TYPE time_frame AS ENUM ('M', 'W', 'D', 'H1', '15m');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'htf_bias') THEN
        CREATE TYPE htf_bias AS ENUM ('engulfing', 'shooting_star', 'hammer', 'flag', 'flat', 'channel');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'atr') THEN
        CREATE TYPE time_frame AS ENUM ('within_atr', 'outside_atr');
    END IF;
END 
$$;


-- accounts
CREATE TABLE IF NOT EXISTS accounts (
    id SERIAL PRIMARY KEY,
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

-- order intent
CREATE TABLE IF NOT EXIST order_intent (
    id UUID PRIMARY KEY,
    symbol VARCHAR(20) NOT NULL,
    setup VARCHAR(20) NOT NULL,
    side trade_type NOT NULL,
    stop_loss NUMERIC(10,5) DEFAULT NULL,
    take_profit NUMERIC(10,5) DEFAULT NULL,
    sent_at TIMESTAMPTZ DEFAULT NULLL, -- To use 'CURRENT TIMESTAMP' would cause false data. Field calculated in program. #adr002
);

-- orders
    -- Transformed orders table from api response.
CREATE TABLE IF NOT EXIST orders (
    id UUID PRIMARY KEY,
    account_id UUID REFERENCES accounts(id) NOT NULL
    order_intent_id UUID REFERENCES order_intent(id) NOT NULL,
    symbol VARCHAR (20) NOT NULL,
    qty NUMERIC(5,2) DEFAULT NULL,
    side trade_type NOT NULL,
    entry_price NUMERIC(10,5) NOT NULL,
    status order_status NOT NULL, -- make enum for order(status) #done
    filled_quantity NUMERIC(5,2) NOT NULL,
    pending_price NUMERIC(10,5) DEFAULT NULL,
    expire_date TIMESTAMPTZ DEFAULT NULL,
    created_date TIMESTAMPTZ DEFAULT NOT NULL, -- To use 'CURRENT TIMESTAMP' would cause false data. Field retrieved from api. #adr002
    last_modified TIMESTAMPTZ DEFAULT NOT NULL, -- To use 'CURRENT TIMESTAMP' would cause false data. Field retrieved from api. #adr002
    is_open BOOLEAN NOT NULL, 
    broker_position_id VARCHAR(50) NOT NULL,
    broker_order_id VARCHAR(50) NOT NULL, -- Found in response of place order POST
    stop_loss_price NUMERIC(10,5) DEFAULT NULL,
    take_profit_price NUMERIC(10,5) DEFAULT NULL,
);

-- positions
CREATE TABLE IF NOT EXISTS positions (
    id UUID PRIMARY KEY,
    account_id UUID REFERENCES accounts(id) NOT NULL,
    order_id UUID REFERENCES orders(id) NOT NULL,
    order_intent_id UUID REFRENCES order_intent(id) NOT NULL,
    symbol VARCHAR(20) NOT NULL,
    side trade_type NOT NULL,
    entry_price NUMERIC(10,5) NOT NULL,
    stop_loss_price NUMERIC(10,5) DEFAULT NULL,
    qty NUMERIC(5,2) NOT NULL,
    take_profit_price NUMERIC(10,5) DEFAULT NULL,
    unrealized_pnl NUMERIC(10,5) NOT NULL,
    entry_time TIMESTAMPTZ NOT NULL,
);

-- trades
CREATE TABLE IF NOT EXISTS trades (
    id SERIAL PRIMARY KEY,
    account_id INTEGER NOT NULL REFERENCES accounts(account_id),
    symbol VARCHAR(30) NOT NULL,
    side trade_type NOT NULL,
    setup VARCHAR(25) DEFAULT NULL,
    lot_size NUMERIC(5, 2) NOT NULL,
    open_time TIMESTAMPTZ NOT NULL,
    entry_price NUMERIC(10, 5) NOT NULL,
    order_type order_type NOT NULL,
    initial_stop_loss NUMERIC(10, 5) DEFAULT NULL,
    initial_take_profit NUMERIC(10, 5) DEFAULT NULL,
    close_time TIMESTAMPTZ DEFAULT NULL,
    close_price NUMERIC(10, 5) DEFAULT NULL,
    commission NUMERIC DEFAULT NULL,
    swap NUMERIC DEFAULT NULL,
    gross_profit NUMERIC DEFAULT NULL,
    net_profit NUMERIC DEFAULT NULL,
    source data_source NOT NULL,
    time_frame time_frame NOT NULL,
    htf_bias htf_bias DEFAULT NULL,
    atr atr DEFAULT NULL,
    order_id UUID REFERENCES orders(id) NOT NULL,
    order_intent_id UUID REFRENCES order_intent(id) NOT NULL,
    position_id UUID REFERENCES positions(id) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

