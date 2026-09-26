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

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'account_tier') THEN
        CREATE TYPE account_tier AS ENUM ('tier_1', 'tier_2', 'tier_3');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'side') THEN
        CREATE TYPE side AS ENUM ('buy', 'sell');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'order_type') THEN
        CREATE TYPE order_type AS ENUM ('limit', 'market_execution', 'stop');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'stops_reason') THEN
        CREATE TYPE stops_reason AS ENUM ('level_break', 'discretionary');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'order_state') THEN
        CREATE TYPE order_state AS ENUM ('placed', 'pending', 'filled', 'rejected', 'canceled', 'expired');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'position_state') THEN
        CREATE TYPE position_state AS ENUM ('open', 'closed');
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'time_frame') THEN
        CREATE TYPE time_frame AS ENUM ('M', 'W', 'D', '4H', 'H1', '15m');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'htf_bias') THEN
        CREATE TYPE htf_bias AS ENUM ('engulfing', 'shooting_star', 'hammer', 'flag', 'flat', 'channel');
    END IF;


    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'idea_state') THEN
        CREATE TYPE idea_state AS ENUM ('planned', 'active', 'closed', 'canceled');
    END IF;
END 
$$;

-- tiers
-- risk tiers
CREATE TABLE IF NOT EXISTS tiers (
    tier account_tier PRIMARY KEY,
    risk_pct NUMERIC(3,1) NOT NULL
);

-- DEFAULT tier data
INSERT INTO tiers (tier, risk_pct)
VALUES ('tier_1', 0.3), ('tier_2', 0.2), ('tier_3', 0.1)
ON CONFLICT (tier) DO NOTHING;

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
    tier account_tier REFERENCES tiers(tier) NOT NULL,
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
-- updated from orders table from api response.
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
    broker_expires_at TIMESTAMPTZ DEFAULT NULL,
    broker_created_at TIMESTAMPTZ NOT NULL, -- To use 'CURRENT TIMESTAMP' would cause false data. Field retrieved from api. #adr002
    broker_modified_at TIMESTAMPTZ NOT NULL, -- To use 'CURRENT TIMESTAMP' would cause false data. Field retrieved from api. #adr002
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
    broker_modified_at TIMESTAMPTZ NOT NULL,
    recorded_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP NOT NULL,
    CONSTRAINT osh_unique UNIQUE (order_id, order_state, broker_modified_at)
);


-- positions
-- what the api confirms about a position
CREATE TABLE IF NOT EXISTS positions (
    id UUID PRIMARY KEY,
    account_id UUID REFERENCES accounts(id) NOT NULL,
    order_id UUID REFERENCES orders(id) UNIQUE NOT NULL,
    position_state position_state NOT NULL,
    symbol VARCHAR(20) NOT NULL,
    side side NOT NULL,
    entry_price NUMERIC(10,5) NOT NULL,
    entry_time TIMESTAMPTZ NOT NULL,
    stop_loss_price NUMERIC(10,5) DEFAULT NULL,
    qty NUMERIC(5,2) NOT NULL,
    take_profit_price NUMERIC(10,5) DEFAULT NULL,
    close_price NUMERIC(10,5) DEFAULT NULL,
    close_time TIMESTAMPTZ DEFAULT NULL,
    gross_profit NUMERIC(10,5) DEFAULT NULL,
    commission NUMERIC(10,5) DEFAULT NULL,
    swap NUMERIC (10,5) DEFAULT NULL,
    CONSTRAINT position_state_oc CHECK (
        (position_state = 'closed' AND gross_profit IS NOT NULL AND close_price IS NOT NULL AND close_time IS NOT NULL)
        OR
        (position_state <> 'closed' AND gross_profit IS NULL AND close_price IS NULL AND close_time IS NULL)
    )
);

-- stop_loss_history
-- history of the stop loss adjustments during the liftime of an open position
CREATE TABLE IF NOT EXISTS stop_loss_history (
    id UUID PRIMARY KEY,
    position_id UUID REFERENCES positions(id) NOT NULL,
    stop_loss NUMERIC(10,5) NOT NULL,
    reason stops_reason NOT NULL,
    sent_at TIMESTAMPTZ NOT NULL,
    broker_modified_at TIMESTAMPTZ DEFAULT NULL 
);
