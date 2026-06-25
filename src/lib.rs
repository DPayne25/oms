use std::net::TcpStream;
use std::sync::Mutex;
use std::{collections::HashMap, error::Error, sync::atomic::{AtomicU64}};
use serde::{Serialize, Deserialize};
use reqwest::{Client};

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


//Research research a crate later https://library.tradingtechnologies.com/tt-fix/ & https://www.onixs.biz/fix-dictionary/4.4/msgType_8_8.html#
pub struct FixAdapter { 
    pub stream: Mutex<Option<TcpStream>>, //validates connection > use single pipeline
    pub begin_string: String, // FIX 4.2 or FIX 4.4
    pub sender_comp_id: String,
    pub target_comp_id: String,
    pub msg_seq_num: AtomicU64,
}

pub struct AdapterError {
    pub error_code: u32, 
    pub error_message: [u8; 64],
}

pub trait BrokerAdapter {
    fn execute_order(&self, account: &AccountConfig, child: &ChildExecution) -> Result<ChildExecution, AdapterError>;
    fn query_status(&self, child_id: &[u8; 16]) -> Result<ExecutionStatus, AdapterError>;
}

//impl BrokerAdapter for FixAdapter {}

#[derive(Deserialize)]
pub struct TradeLockerConfig {
    pub tl_url: String,
    pub tl_email: String,
    pub tl_password: String,
    pub tl_server: String,
    pub tl_account_id: i64,
    pub tl_acc_num: i8,
}

#[derive(Serialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub server: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expire_date: String,
}

impl LoginRequest {
    
    pub async fn tl_login(&self) -> Result<TokenResponse, Box<dyn Error>> {
        let url = "https://demo.tradelocker.com/backend-api/auth/jwt/token";
        let res = Client::new().post(url).json(self).send().await?;
        let token_out: TokenResponse = res.json().await?;

        Ok(token_out)
    }
}

impl TokenResponse {
    pub fn token (&self) -> String {
        format!("Bearer {}", self.access_token)
    }
}

pub fn load_config(path: &str) -> Result<HashMap<String, TradeLockerConfig>, Box<dyn Error>> {
    let contents = std::fs::read_to_string(path)?;
    let accounts: HashMap<String, TradeLockerConfig> = toml::from_str(&contents)?;
    Ok(accounts)
}
