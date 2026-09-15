CREATE TABLE IF NOT EXISTS oreder_intent (
    id UUID PRIMARY KEY,
    vantage_id UUID NOT NULL REFERENCE vantage,
    symbol VARCHAR(20) NOT NULL,
    setup VARCHAR(20) NOT NULL,
    side trade_type NOT NULL,
    stop_loss NUMERIC (10, 5) DEFAULT NULL,
    take_profit NUMERIC (10, 5) DEFAULT NULL,
    sent_at TIMESTAMPTZ DEFAULT NULL -- DEFAULT NULL because CURRENT TIMESTAMP would be a false assumption. TIMESTAMP needs t obe established in the program.
);


