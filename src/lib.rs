// Order Direction
pub enum OrderDirection {
    Buy,
    Sell,
}

impl OrderDirection {
    pub fn to_fix_byte (&self) -> u8 {
        match self {
            Self::Buy => b'B',
            Self::Sell => b'S',
        }
    }
}

// Order Intent
pub enum OrderIntent {
    Market{
        parent_id: [u8; 16],
        symbol: [u8; 8],
        direction: OrderDirection,
        stop_loss: u64,
        take_profit: u64,
        risk_percent: u32
    },
    Pending {
        entry_price: u64,
        parent_id: [u8; 16],
        symbol: [u8; 8],
        direction: OrderDirection,
        stop_loss: u64,
        take_profit: u64,
        risk_percent: u32
    },
}

// Account Config
pub enum BrokerPlatform {
    Oanda,
    TradeLocker,
    StoneX,
    Mt5,
}
pub struct AccountConfig {
    account_id: [u8; 16],
    balance: u64,
    platform: BrokerPlatform,
}

// Child Execution
pub enum ExecutionStatus {
    Pending,
    Cancelled,
    Filled,
    Rejected,
    Expired,
    Closed,
}
pub struct ChildExecution {
    parent_id: [u8; 16],
    child_id: [u8; 16],
    account_id: [u8; 16],
    lot_size: u64,
    status: ExecutionStatus,
}
