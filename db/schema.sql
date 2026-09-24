-- traders
CREATE TABLE IF NOT EXISTS traders (
    id UUID PRIMARY KEY,
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

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'data_source') THEN
        CREATE TYPE data_source AS ENUM ('live', 'imported', 'manual');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'order_type') THEN
        CREATE TYPE order_type AS ENUM ('limit', 'market_execution', 'stop');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'stops_reason') THEN
        CREATE TYPE stops_reason AS ENUM ('level_break', 'atr_trail', 'manual');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'order_state') THEN
        CREATE TYPE order_state AS ENUM ('placed', 'pending', 'filled', 'rejected', 'canceled', 'expired');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'position_state') THEN
        CREATE TYPE position_state AS ENUM ('open', 'closed', 'liquidating');
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'time_frame') THEN
        CREATE TYPE time_frame AS ENUM ('M', 'W', 'D', '4H', 'H1', '15m');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'htf_bias') THEN
        CREATE TYPE htf_bias AS ENUM ('engulfing', 'shooting_star', 'hammer', 'flag', 'flat', 'channel');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'atr') THEN
        CREATE TYPE atr AS ENUM ('within_atr', 'outside_atr');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'idea_state') THEN
        CREATE TYPE idea_state AS ENUM ('planned', 'active', 'closed', 'missed');
    END IF;
END 
$$;


-- accounts
-- list of user accounts
CREATE TABLE IF NOT EXISTS accounts (
    id UUID PRIMARY KEY,
    currency VARCHAR(3) NOT NULL DEFAULT 'USD',
    account_number VARCHAR(255) UNIQUE NOT NULL,
    broker_name VARCHAR(255) NOT NULL,
    platform_name VARCHAR(255) NOT NULL,
    leverage INTEGER NOT NULL,
    trader_id UUID NOT NULL REFERENCES traders(id),
    role account_role NOT NULL,
    capital_source capital_source NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP    
);

-- idea
-- prerequisite for order_intent
CREATE TABLE IF NOT EXISTS ideas (
    id UUID PRIMARY KEY,
    trader_id UUID NOT NULL REFERENCES traders(id),
    symbol VARCHAR(20) NOT NULL,
    side side NOT NULL,
    setup VARCHAR(20) NOT NULL,
    planned_entry NUMERIC(10, 5) NOT NULL,
    stop_loss NUMERIC(10, 5) NOT NULL,
    take_profit NUMERIC(10, 5) NOT NULL,
    idea_state idea_state NOT NULL,
    time_frame time_frame NOT NULL,
    htf_bias htf_bias DEFAULT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    first_fill_at TIMESTAMPTZ DEFAULT NULL,
    closed_at TIMESTAMPTZ DEFAULT NULL
);

-- order intent
-- input from UI
CREATE TABLE IF NOT EXISTS order_intent (
    id UUID PRIMARY KEY,
    idea_id UUID NOT NULL REFERENCES ideas(id),
    is_pending BOOLEAN NOT NULL,
    pending_price NUMERIC(10,5),
    CONSTRAINT pending_toggle CHECK (
        (is_pending AND pending_price IS NOT NULL)
        OR
        (NOT is_pending AND pending_price IS NULL)
    ),
    sent_at TIMESTAMPTZ  NOT NULL -- To use 'CURRENT TIMESTAMP' would cause false data. Field calculated in program. #adr002
);

-- new orders
-- what the code calculated and sent for place order parameters
CREATE TABLE IF NOT EXISTS new_orders (
    id UUID PRIMARY KEY,
    account_id UUID REFERENCES accounts(id) NOT NULL,
    order_intent_id UUID REFERENCES order_intent(id) NOT NULL,
    qty NUMERIC(5,2) NOT NULL,
    side side NOT NULL,
    price NUMERIC(10,5) DEFAULT NULL,
    stop_loss NUMERIC(10,5) NOT NULL,
    take_profit NUMERIC(10,5) NOT NULL,
    kind order_type NOT NULL,
    CONSTRAINT pending_check CHECK (
        (kind = 'market_execution' AND price IS NULL)
        OR
        (kind <> 'market_execution' AND price IS NOT NULL)
    ),
    raw_request JSONB NOT NULL
);

-- orders
-- Updated from orders table from api response.
CREATE TABLE IF NOT EXISTS orders (
    id UUID PRIMARY KEY,
    new_order_id UUID REFERENCES new_orders(id) UNIQUE NOT NULL,
    symbol VARCHAR (20) NOT NULL,
    qty NUMERIC(5,2) DEFAULT NULL,
    side side NOT NULL,
    entry_price NUMERIC(10,5) DEFAULT NULL,
    order_state order_state NOT NULL,
    filled_qty NUMERIC(5,2) DEFAULT NULL,
    pending_price NUMERIC(10,5) DEFAULT NULL,
    expire_date TIMESTAMPTZ DEFAULT NULL,
    created_date TIMESTAMPTZ NOT NULL, -- To use 'CURRENT TIMESTAMP' would cause false data. Field retrieved from api. #adr002
    last_modified TIMESTAMPTZ NOT NULL, -- To use 'CURRENT TIMESTAMP' would cause false data. Field retrieved from api. #adr002
    broker_position_id VARCHAR(50) DEFAULT NULL,
    broker_order_id VARCHAR(50) NOT NULL, -- Found in response of place order POST
    stop_loss_price NUMERIC(10,5) DEFAULT NULL,
    take_profit_price NUMERIC(10,5) DEFAULT NULL
);

-- order state history
-- appendable log of orders state
CREATE TABLE IF NOT EXISTS order_state_history (
    id UUID PRIMARY KEY,
    order_id UUID REFERENCES orders(id) NOT NULL,
    order_state order_state NOT NULL,
    entry_price NUMERIC(10,5) DEFAULT NULL,
    filled_qty NUMERIC(5,2) DEFAULT NULL,
    broker_event_at TIMESTAMPTZ NOT NULL,
    recorded_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMPTZ
);


-- positions
CREATE TABLE IF NOT EXISTS positions (
    id UUID PRIMARY KEY,
    account_id UUID REFERENCES accounts(id) NOT NULL,
    idea_id UUID REFERENCES ideas(id) NOT NULL,
    order_id UUID REFERENCES orders(id) NOT NULL,
    order_intent_id UUID REFERENCES order_intent(id) NOT NULL,
    symbol VARCHAR(20) NOT NULL,
    side side NOT NULL,
    entry_price NUMERIC(10,5) NOT NULL,
    stop_loss_price NUMERIC(10,5) DEFAULT NULL,
    qty NUMERIC(5,2) NOT NULL,
    take_profit_price NUMERIC(10,5) DEFAULT NULL,
    unrealized_pnl NUMERIC(10,5) NOT NULL,
    entry_time TIMESTAMPTZ NOT NULL,
);

-- trades
CREATE TABLE IF NOT EXISTS trades (
    id UUID PRIMARY KEY,
    account_id INTEGER NOT NULL REFERENCES accounts(account_id),
    idea_id UUID REFERENCES ideas(id) NOT NULL,
    symbol VARCHAR(30) NOT NULL,
    side side NOT NULL,
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
    order_intent_id UUID REFERENCES order_intent(id) NOT NULL,
    position_id UUID REFERENCES positions(id) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

