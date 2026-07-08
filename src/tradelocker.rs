use std::{str::FromStr, collections::HashMap, error::Error};
use serde::{Serialize, Deserialize};
use reqwest::Client;
use rust_decimal::{prelude::FromPrimitive, Decimal};
use rust_decimal_macros;
use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct TLAccountState {
    pub config: TradeLockerConfig,
    pub token: Option<TokenResponse>,
    pub account_info: Option<AccountInfo>,
    pub instruments: Option<Vec<InstrumentInfo>>,
    pub orders_history: Option<Vec<Vec<serde_json::Value>>>,
    pub tl_config: Option<ConfigData>,
    pub zipped_orders: Option<Vec<HashMap<String, serde_json::Value>>>,
}

impl TLAccountState {
    pub async fn ensure_fresh_token(&mut self, client: &Client) -> Result<(), Box<dyn Error>> {
        match &self.token {
            None => {
                let login = LoginRequest {
                    email: self.config.tl_email.clone(),
                    password: self.config.tl_password.clone(),
                    server: self.config.tl_server.clone(),
                };
                let new_token = login.tl_login(client).await?;
                self.token = Some(new_token);
            }
            Some(token) => {
                if let Some(new_token) = token.check_token(client).await? {
                    self.token = Some(new_token);
                }
            }
        }
        Ok(())
    }

    pub async fn fetch_account_info(&mut self, client: &Client) -> Result<(), Box<dyn Error>> {
        let token = self.token.as_ref().ok_or("no token for this account")?;
        let url = "https://demo.tradelocker.com/backend-api/auth/jwt/all-accounts";

        let res = client
            .get(url)
            .bearer_auth(&token.access_token)
            .header("accept", "application/json")
            .send()
            .await?;

        let parsed: AllAccountsResponse = res.json().await?;

        let info = parsed.accounts.into_iter()
            .next()
            .ok_or("no accounts returned")?;

        self.account_info = Some(info);
        Ok(())
    }

    pub async fn fetch_instrument_info(&mut self, client: &Client) -> Result<(), Box<dyn Error>> {
        let token = self.token.as_ref().ok_or("no token for this account")?;
        let account = self.account_info.as_ref().ok_or("no account_id found for this account")?;
        let url = format!("https://demo.tradelocker.com/backend-api/trade/accounts/{}/instruments", account.id);

        let res = client
            .get(url)
            .bearer_auth(&token.access_token)
            .header("accept", "application/json")
            .header("accNum", &account.acc_num)
            .send()
            .await?;

        let parsed: InstrumentResponse = res.json().await?;

        self.instruments = Some(parsed.d.instruments);

        Ok(())
    }

    pub async fn fetch_order_history(&mut self, client: &Client) -> Result<(), Box<dyn Error>> {
        let token = self.token.as_ref().ok_or("no token for this account")?;
        let account = self.account_info.as_ref().ok_or("no account_info for this account")?;
        let url = format!("https://demo.tradelocker.com/backend-api/trade/accounts/{}/ordersHistory", account.id);

        let res = client
            .get(url)
            .bearer_auth(&token.access_token)
            .header("accept", "application/json")
            .header("accNum", &account.acc_num)
            .send()
            .await?;

        let parsed: OrderHistoryResponse = res.json().await?;
        self.orders_history = Some(parsed.d.orders_history);
        Ok(())
    }

    pub async fn fetch_config(&mut self, client: &Client) -> Result<(), Box<dyn Error>> {
        let token = self.token.as_ref().ok_or("no token for this account")?;
        let account = self.account_info.as_ref().ok_or("no account_info for this account")?;
        let url = format!("https://demo.tradelocker.com/backend-api/trade/config");

        let res = client
            .get(url)
            .bearer_auth(&token.access_token)
            .header("accept", "application/json")
            .header("accNum", &account.acc_num)
            .send()
            .await?;

        let parsed: ConfigResponse = res.json().await?;
        self.tl_config = Some(parsed.d);
        
        Ok(())
    }

    pub fn zip_orders(&self) -> Result<Vec<HashMap<String, serde_json::Value>>, Box<dyn Error>> {
        let columns = &self.tl_config
            .as_ref()
            .ok_or("tl_config not loaded")?
            .orders_history_config
            .columns;
        let rows = self.orders_history
            .as_ref()
            .ok_or("ordres_history not loadeed")?;

        let zipped = rows
            .iter()
            .map(|row| {
                columns
                    .iter()
                    .zip(row.iter())
                    .map(|(col, val)| (col.id.clone(), val.clone()))
                    .collect::<HashMap<String, serde_json::Value>>()
            })
            .collect();
        
        Ok(zipped)
    }

    pub fn group_position_id(&self) -> Result<HashMap<String, Vec<HashMap<String, serde_json::Value>>>, Box<dyn Error>> {
        let zipped_orders = self.zipped_orders
            .as_ref()
            .ok_or("zipped_orders not loaded")?;

        let mut grouped: HashMap<String, Vec<HashMap<String, serde_json::Value>>> = HashMap::new();
        
        for row in zipped_orders {
            let position_id = match row.get("positionId").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => continue,
            };
            grouped
                .entry(position_id)
                .or_insert_with(Vec::new)
                .push(row.clone());
        }
        Ok(grouped)
    }

    //find tradable_intstrument_id
    pub fn find_static_instrument_info(&self, instrument_name: &str) -> Result<(i64, i64), Box<dyn Error>> {
        let instruments = self.instruments.as_ref().ok_or("instrument data error")?;
        let instrument = instruments.iter()
            .find(|inst| inst.name == instrument_name)
            .ok_or_else(|| format!("no routes defined for instrument {}", instrument_name))?;

        let route = instrument.routes.first()
            .ok_or_else(|| format!("no routes defined for instrument {}", instrument_name))?;

        let instrument_route_id_tr_instrument_id = (route.id, instrument.tradable_instrument_id);

        Ok(instrument_route_id_tr_instrument_id)
    }

    //Account Balance
    pub fn find_account_balance(&self) -> Result<Decimal, Box<dyn Error>> {
        let acc_info = self.account_info.as_ref().ok_or("no account_info")?;

        let balance:Decimal = Decimal::from_str(&acc_info.account_balance)?;

        Ok(balance)
    }

    pub async fn get_current_prices(&self, client: &Client, instrument_name: &str) -> Result<CurrentPrices, Box<dyn Error>> {
        let token = self.token.as_ref().ok_or("no token for this account")?;
        let account = self.account_info.as_ref().ok_or("no account_info for this account")?;
        let (route_id, instrument_id) = self.find_static_instrument_info(instrument_name)?;


        let url = format!("https://demo.tradelocker.com/backend-api/trade/quotes?routeId={}&tradableInstrumentId={}", route_id, instrument_id);

        let prices = client
            .get(url)
            .bearer_auth(&token.access_token)
            .header("accept", "application/json")
            .header("accNum", &account.acc_num)
            .send()
            .await?;

        Ok(prices)
    }

}


#[derive(Deserialize, Debug)]
pub struct TradeLockerConfig {
    pub tl_url: String, // duplicated
    pub tl_email: String,
    pub tl_password: String,
    pub tl_server: String, // duplicated
    pub tl_account_id: String,
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
    pub async fn tl_login(&self, client: &Client) -> Result<TokenResponse, Box<dyn Error>> {
        let url = "https://demo.tradelocker.com/backend-api/auth/jwt/token";
        let res = client.post(url).json(self).send().await?;
        let token_out: TokenResponse = res.json().await?;

        Ok(token_out)
    }
}

#[derive(Serialize)]
pub struct RefreshRequest {
    refresh_token: String,
}

impl TokenResponse {
    pub fn bearer_auth (&self) -> Result<String, Box<dyn Error>> {
        let bearer_auth = format!("Bearer {}", self.access_token);
        Ok(bearer_auth)
    }

    pub async fn check_token(&self, client: &Client) -> Result<Option<TokenResponse>, Box<dyn Error>> {

        let expire_date = DateTime::parse_from_rfc3339(&self.expire_date)?;
        let now = Utc::now();
        let seconds_remaining = (expire_date.with_timezone(&Utc) - now).num_seconds();

        if seconds_remaining > 300 {
            return Ok(None);
        }

        let url = "https://demo.tradelocker.com/backend-api/auth/jwt/refresh";

        let payload = RefreshRequest {
            refresh_token: self.refresh_token.clone(),
        };

        let res = client.post(url).json(&payload).send().await?;
        let new_token: TokenResponse = res.json().await?;

        Ok(Some(new_token))
    }
}

// Account Details
#[derive(Deserialize, Debug)]
pub struct AllAccountsResponse {
    accounts: Vec<AccountInfo>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    pub id: String,
    pub name: String,
    pub currency: String,
    pub acc_num: String,
    pub account_balance: String,
    pub status: String,
}

// List of Instruments
#[derive(Deserialize, Debug)]
pub struct InstrumentResponse {
    pub d: InstrumentData,
}

#[derive(Deserialize, Debug)]
pub struct InstrumentData {
    pub instruments: Vec<InstrumentInfo>
}

#[derive(Deserialize, Debug)]
pub struct Routes {
    pub id: i64,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SymbolType {
    Crypto,
    Equity,
    EquityCfd,
    Etf,
    Forex,
    Futures,
    FuturesCfd,
    Indicies,
    Options,
    Spreadbet,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentInfo {
    pub id: i32,
    pub routes: Vec<Routes>,
    pub market_data_exchange: String,
    pub name: String,
    pub tradable_instrument_id: i64,
    pub trading_exchange: String,
    #[serde(rename = "type")]
    pub kind: SymbolType,
}

// Order History
#[derive(Deserialize, Debug)]
pub struct OrderHistoryResponse {
    pub d: OrderHistoryData
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]

pub struct OrderHistoryData {
    pub orders_history: Vec<Vec<serde_json::Value>>,
    pub has_more: bool,
}

// Config
#[derive(Deserialize, Debug)]
pub struct ConfigResponse {
    pub d: ConfigData,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConfigData {
    pub orders_history_config: OrdersHistoryConfig,

    /*
    pub account_details_config: AccountDetailsConfig,
    pub customer_access: Vec<ColumnDef>,
        pub filled_orders_config: Vec<ColumnDef>,
        pub orders_config: Vec<ColumnDef>,
        pub poisitions_config: Vec<ColumnDef>,
        pub rate_limits: Vec<ColumnDef>,
        pub limits: Vec<ColumnDef>,
    */
}
/*
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AccountDetailsConfig {
    pub orders: bool,
    pub orders_history: bool,
    pub filled_orders: bool,
    pub positions: bool,
    pub symbol_info: bool,
    pub market_depth: bool,
}
*/
#[derive(Deserialize, Debug)]
pub struct AccountDetailsConfig {
    pub columns: Vec<ColumnDef>,
}

#[derive(Deserialize, Debug)]
pub struct OrdersHistoryConfig {
    pub columns: Vec<ColumnDef>,
}

#[derive(Deserialize, Debug)]
pub struct ColumnDef {
    pub id: String,
}

//Zipped Orders
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZippedOrders {
    id: String,
    tradable_instrument_id: String,
    route_id: String,
    qty: String,
    side: String,
    #[serde(rename = "type")]
    kind: String,
    status: String,
    filled_qty: String,
    avg_price: String,
    price: String,
    stop_price: String,
    validity: String,
    expire_date: String,
    created_date: String,
    last_modified: String,
    is_open: String,
    position_id: String,
    stop_loss: String,
    stop_loss_type: String,
    take_profit: String,
    take_profit_type: String,
    strategy_id: String,
}

//Trades (Transformed Data That will be INSERTed into db)
#[derive(Serialize)]
pub struct Trades {
    pub trade_id: i64,
    pub account_id: i64,
    pub symbol: String,
    pub side: String,
    pub setup: String,
    pub lot_size: Decimal,
    pub open_time: String,
    pub open_price: Decimal,
    pub initial_stop_loss: Decimal,
    pub close_time: String,
    pub close_price: Decimal,
    pub sl_was_modified: bool,
    pub tp_was_modified: bool,
    pub commission: Decimal,
    pub gross_profit: Decimal,
    pub net_profit: Decimal,
    pub status: String,
    pub source: String,
}

//Place New Order
#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StopLossType {
    Absolute,
    Offset,
    TrailingOffset,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TakeProfitType {
    Absolute,
    Offset,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderType {
    Limit,
    Market,
    Stop,
}

#[derive(Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Validity {
    Gtc,
    Ioc,
}

#[derive(Serialize)]
pub enum TradeSetup {
    JCP,
    DipNDot,
    TestSetup
}

impl TradeSetup {
    pub fn trade_risk_percentage(&self) -> Decimal {
        match self {
            TradeSetup::JCP => rust_decimal_macros::dec!(0.002),
            TradeSetup::DipNDot => rust_decimal_macros::dec!(0.0071),
            TradeSetup::TestSetup => rust_decimal_macros::dec!(0.001,)
        }
    }
}

pub trait RiskCalculator {
    fn calculate_money_at_risk(&self, balance: Decimal) -> Decimal;
    fn calculate_pct_to_unit_size(&self, balance: Decimal) -> Decimal;
}

impl RiskCalculator for OrderIntent {
    fn calculate_money_at_risk(&self, balance: Decimal) -> Decimal {
        balance * self.trade_risk_percentage()
    }

    fn calculate_pct_to_unit_size(&self, balance: Decimal) -> Decimal {
        //riskbalance * self.trade_risk_percentage()
        let pip_distance = current_price() - stop_loss;
        (balance * self.trade_risk_percentage()) * (pip_distance.abs() * pip_value())
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CurrentPrices {
    pub ap: Decimal, //best ask price
    pub _as: Decimal, //best ask size
    pub bp: Decimal, //best bid price
    pub bs: Decimal, //best ask size
}

#[derive(Serialize)]
pub struct OrderIntent {
    //pub price: Option<Decimal>, //Change to DECIMAL
    pub setup: TradeSetup,
    pub side: OrderSide,
    pub stop_loss: Decimal,
    pub take_profit: Decimal,
}


#[derive(Serialize)]
pub struct NewOrder {
    //pub price: Option<Decimal>, //Change to DECIMAL
    pub qty: Decimal,
    pub route_id: i64,
    pub side: String,
    //pub strategy_id:Opition<String>,
    pub stop_loss: Decimal,
    pub stop_loss_type: String,
    //pub stop_price: Option<Decimal>,
    pub take_profit: Decimal,
    pub take_profit_type: String,
    //pub tr_stop_offset: i64,
    pub tradable_instrument_id: i64,
    #[serde(rename = "type")]
    pub kind: String,
    pub validity: String,
}

pub trait OrderExecutor {
    async fn place_new_order(&self, account: &TLAccountState, client: &Client) -> Result<serde_json::Value, Box<dyn Error>>;
}

impl OrderExecutor for NewOrder {
    async fn place_new_order(&self, accounts: &TLAccountState, client: &Client) -> Result<serde_json::Value, Box<dyn Error>> {
        let token = accounts.token.as_ref().ok_or("no token for this account")?;
        let account = accounts.account_info.as_ref().ok_or("no account_id found for this account")?;
                
        let url = format!("https://demo.tradelocker.com/backend-api/trade/accounts/{}/orders", account.id);
       
        let res = client
            .post(url)
            .bearer_auth(&token.access_token)
            .json(&self)
            .header("accept", "application/json")
            .header("accNum", &account.acc_num)
            .header("content-type", "application/json")
            .send()
            .await?;

        let status = res.status();

        let response_payload: serde_json::Value = res.json().await?;
        
        println!("{}, {}", status, response_payload);
        
        Ok(response_payload)

    }
}

//Free Functions

//`load_config()` stores account login values into `TLAccountState` struct
pub fn load_config(path: &str) -> Result<HashMap<String, TLAccountState>, Box<dyn Error>> {
    let contents = std::fs::read_to_string(path)?;
    let configs: HashMap<String, TradeLockerConfig> = toml::from_str(&contents)?;
    let mut accounts: HashMap<String, TLAccountState> = HashMap::new();

    for (name, config) in configs {
        accounts.insert(name, TLAccountState {config, token: None, account_info: None, instruments: None, orders_history: None, tl_config: None, zipped_orders: None});
    }
    Ok(accounts)
}

pub async fn ensure_all_fresh(accounts: &mut HashMap<String, TLAccountState>, client: &Client) {
    // Login to accounts sourced from config.toml
    for (account_name, account) in accounts.iter_mut() {
        match account.ensure_fresh_token(client).await {
            Ok(()) => println!("{} token ready", account_name),
            Err(e) => println!("{} failed: {}", account_name, e),
        }
    }
}

//pub async fn place_new_order() -> Result<(), Box<dyn Error>> {}