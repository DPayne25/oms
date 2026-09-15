use serde::{Debug, Deserialize, Serialize};

//Account States:
//Declares the names of the state that any account can be in at any given time.  
#[derive(Debug, Deserialize, Serialize)]
pub enum AccountState {
    Locked,
    Unlocked,
}
impl AccountState {
    pub fn state_transition (equity: Decimal, hwm: Decimal) -> Self {
        match hwm - equity {
            drawdown if drawdown >= max_drawdown => Self::Locked,
            _ => Self::Unlocked,
        }
    }
}

//Order States: 
//Declares the name of the state that any order can be in at any given time.
#[derive(Debug, PartialEq)]
pub enum OrderState {
    Rejected,
    Filled,
    Pending,
    Canceled,
    Expired,
}
impl OrderState {
    pub fn state_transition (status: &str) -> Option<Self> {
        match status {
            "REJECTED" | "Rejected" | "rejected" => Self::Rejected,
            "FILLED" | "Filled" | "filled" => Self::Filled,
            "PENDING" | "Pending" | "pending" => Self::Pending,
            "CANCELED" | "Canceled" | "canceled" => Self::Canceled,
            "EXPIRED" | "Expired" | "expired" => Self::Expired,
            _ => None,
        }
    }
}


//Position States:
//Declares the names of the stats that any position can be in at any given time.
pub enum Position State {
    Open,
    Liquidating,
    Closed,
}
impl Position {
    pub fn state_transition (status: &str) -> Option<Self> {
        match status {
            "OPEN" | "Open" | "open" => Self::Open,
            "LIQUIDATING" | "Liquidating" | "liquidating" => Self::Liquidating,
            "CLOSED" | "Closed" | "closed" => Self::Closed,
            _ => None,
        }
    }
}

