pub mod tradelocker;

pub struct Position {
    entry_price: Decimal,
    qty: Decimal,
    sl_price: Decimal,
    tp_price: Decimal,
}

/// Vantages are Positions grouped by instrument
pub struct Vantage {
    instrument: String,
    setup: String,
    vantage: Vec<Position>,
}

/// ActiveIdeas are a vector of Vantages that are actively open in the maarket meant to be managed/modified identically.
pub struct ActiveIdeas {
    ideas: Vec<Vantage>,
}