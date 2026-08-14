use chrono::{DateTime, Utc};
use dotenvy::dotenv;
use reqwest::Client;
use rust_decimal::Decimal;
use rust_decimal_macros;
use serde::{Deserialize, Serialize};
use serde_json;
use std::{collections::HashMap, env, error::Error, str::FromStr};
//use std::io::{self, Write, Read};
use tokio::time::Duration;

////////////////////////////////////////////////////////////
// Enums
////////////////////////////////////////////////////////////

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

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TradeSetup {
    JCP,
    DipNDot,
    TestSetup,
}

impl TradeSetup {
    pub fn trade_risk_percentage(&self) -> Decimal {
        match self {
            TradeSetup::JCP => rust_decimal_macros::dec!(0.002),
            TradeSetup::DipNDot => rust_decimal_macros::dec!(0.0071),
            TradeSetup::TestSetup => rust_decimal_macros::dec!(0.001,),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}

impl OrderSide {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderSide::Buy => "buy",
            OrderSide::Sell => "sell",
        }
    }
}

impl FromStr for OrderSide {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "buy" => Ok(OrderSide::Buy),
            "sell" => Ok(OrderSide::Sell),
            other => Err(format!("invalid side '{other}', expected 'buy' or 'sell'")),
        }
    }
}

#[derive(Deserialize, Debug)]
pub enum RateLimitType {
    #[serde(rename = "GET_ACCOUNTS")]
    GetAccounts,
    #[serde(rename = "GET_EXECUTIONS")]
    GetExecutions,
    #[serde(rename = "PLACE_ORDER")]
    PlaceOrder,
    #[serde(rename = "GET_INSTRUMENTS")]
    GetInstuments,
    #[serde(rename = "GET_ORDERS")]
    GetOrders,
    #[serde(rename = "GET_ORDERS_HISTORY")]
    GetOrdersHistory,
    #[serde(rename = "GET_POSITIONS")]
    GetPositions,
    #[serde(rename = "GET_ACCOUNTS_STATE")]
    GetAccountsState,
    #[serde(rename = "GET_INSTRUMENT_DETAILS")]
    GetInstrumentDetails,
    #[serde(rename = "GET_TRADE_SESSIONS")]
    GetTradeSessions,
    #[serde(rename = "GET_SESSION_STATUSES")]
    GetSessionStatuses,
    #[serde(rename = "MODIFY_ORDER")]
    ModifyOrder,
    #[serde(rename = "MODIFY_POSITION")]
    ModifyPosition,
    #[serde(rename = "DAILY_BAR")]
    DailyBar,
    #[serde(rename = "QUOTES")]
    Quotes,
    #[serde(rename = "DEPTH")]
    Depth,
    #[serde(rename = "TRADES")]
    Trades,
    #[serde(rename = "QUOTES_HISTORY")]
    QuotesHistory,
}

#[derive(Deserialize, Debug)]
pub enum LimitType {
    #[serde(rename = "QUOTES_HISTORY_BARS")]
    QuotesHistoryBars,
    #[serde(rename = "MAX_ORDERS_COUNT_IN_HISTORY")]
    MaxOrdersCountInHistory,
}

#[derive(Deserialize, Debug)]
pub enum RateLimitMeasure {
    #[serde(rename = "SECONDS")]
    Seconds,
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

////////////////////////////////////////////////////////////
// Functions
////////////////////////////////////////////////////////////

//`load_config()` stores account login values into `TLAccountState` struct
pub fn load_config(path: &str) -> Result<HashMap<String, TLAccountState>, Box<dyn Error>> {
    let contents = std::fs::read_to_string(path)?;
    let configs: HashMap<String, TradeLockerConfig> = toml::from_str(&contents)?;
    let mut accounts: HashMap<String, TLAccountState> = HashMap::new();

    for (name, config) in configs {
        accounts.insert(
            name,
            TLAccountState {
                config,
                token: None,
                account_info: None,
                instruments: None,
                orders_history: None,
                config_data: None,
                zipped_orders: None,
                quotes: None,
            },
        );
    }
    Ok(accounts)
}

pub async fn refresh_all_tokens(accounts: &mut HashMap<String, TLAccountState>, client: &Client) {
    // Login to accounts sourced from config.toml
    for (account_name, account) in accounts.iter_mut() {
        if let Err(e) = account.ensure_fresh_token(client).await {
            println!("{} token refresh failed: {}", account_name, e);
        }
    }
}

//pub async fn place_new_order() -> Result<(), Box<dyn Error>> {}

//GET all TradeLocker account info
pub async fn get_all_account_info(accounts: &mut HashMap<String, TLAccountState>, client: &Client) {
    for (account_name, account) in accounts.iter_mut() {
        if let Err(e) = account.fetch_account_info(&client).await {
            println!("{} failed to fetch account info: {}", account_name, e);
        }

        if let Err(e) = account.fetch_instrument_info(&client).await {
            println!("{} failed to fetch instrument info: {}", account_name, e);
            if let Some(source) = e.source() {
                println!(" caused by: {}", source);
            }
        }
    }
}

//GET all TradeLocker order history info
pub async fn get_all_order_history_info(
    accounts: &mut HashMap<String, TLAccountState>,
    client: &Client,
) {
    for (account_name, account) in accounts.iter_mut() {
        if let Err(e) = account.fetch_order_history(&client).await {
            println!("{} failed to fetch order history info: {}", account_name, e);
            if let Some(source) = e.source() {
                println!(" caused by: {}", source);
            }
        }
    }
}

//GET api config headers
pub async fn get_configuration(accounts: &mut HashMap<String, TLAccountState>, client: &Client) {
    for (account_name, account) in accounts.iter_mut() {
        if let Err(e) = account.fetch_config(&client).await {
            println!("{} failed to fetch config info: {}", account_name, e);
            if let Some(source) = e.source() {
                println!(" caused by: {}", source);
            }
        }
    }
}

//Zip config headers with order history
pub async fn zip_all_order_history(accounts: &mut HashMap<String, TLAccountState>) {
    for (account_name, account) in accounts.iter_mut() {
        match account.zip_orders() {
            Ok(zipped) => {
                account.zipped_orders = Some(zipped);

                match account.group_position_id() {
                    Ok(grouped) => {
                        println!("{}: {} grouped positions", account_name, grouped.len());
                    }
                    Err(e) => println!("{} failed to group: {}", account_name, e),
                }
            }
            Err(e) => println!("{} failed to zip orders: {}", account_name, e),
        }
    }
}

pub async fn build_order_intent(
    instrument: String,
    side: OrderSide,
    setup: TradeSetup,
    stop_loss: Decimal,
    take_profit: Decimal,
) -> OrderIntent {
    let order_intent: OrderIntent = OrderIntent {
        instrument: instrument,
        setup: setup,
        side: side,
        stop_loss: stop_loss,
        take_profit: take_profit,
    };
    return order_intent;
}

/*
pub async fn new_order(accounts: &mut HashMap<String, TLAccountState>, client: &Client, intent: &OrderIntent) -> Result<(), Box<dyn Error>> {
    for (name, account) in accounts.iter() {
        let order = account.build_new_order(&intent, &client).await?;
        match account.place_new_order(&order, &client).await {
            Ok(resp) => println!("{name}: {resp}"),
            Err(e)   => println!("{name} failed: {e}"),
        }
    }
    Ok(())
}
*/

/*
pub fn build_order_intent() -> Result<OrderIntent, Box<dyn Error>> {
    let instrument = prompt("Instrument (e.g. AUDCAD): ")?;

    let side_input = prompt("Side (buy/sell): ")?;
    let side = side_input.parse::<OrderSide>()?;

    let stop_loss_input = prompt("Stop loss (e.g. 0.98478): ")?;
    let stop_loss = Decimal::from_str(&stop_loss_input)?;

    let take_profit_input = prompt("Take profit (e.g. 0.97833): ")?;
    let take_profit = Decimal::from_str(&take_profit_input)?;

    Ok(OrderIntent {
        instrument,
        setup: TradeSetup::TestSetup,
        side,
        stop_loss,
        take_profit,
    })
}

pub fn prompt(label: &str) -> Result<String, Box<dyn Error>> {
    print!("{label}");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_string())
}
    */

////////////////////////////////////////////////////////////
// Structs & Methods
////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct TLAccountState {
    pub config: TradeLockerConfig,
    pub token: Option<TokenResponse>,
    pub account_info: Option<AccountInfo>,
    pub instruments: Option<Vec<InstrumentInfo>>,
    pub orders_history: Option<Vec<Vec<serde_json::Value>>>,
    pub config_data: Option<ConfigData>,
    pub zipped_orders: Option<Vec<HashMap<String, serde_json::Value>>>,
    pub quotes: Option<QuotesResponse>,
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
                let new_token = login.fetch_jwt_token(client).await?;
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

        let info = parsed
            .accounts
            .into_iter()
            .next()
            .ok_or("no accounts returned")?;

        self.account_info = Some(info);
        Ok(())
    }

    pub async fn fetch_instrument_info(&mut self, client: &Client) -> Result<(), Box<dyn Error>> {
        let token = self.token.as_ref().ok_or("no token for this account")?;
        let account = self
            .account_info
            .as_ref()
            .ok_or("no account_id found for this account")?;
        let url = format!(
            "https://demo.tradelocker.com/backend-api/trade/accounts/{}/instruments",
            account.id
        );

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
        let account = self
            .account_info
            .as_ref()
            .ok_or("no account_info for this account")?;
        let url = format!(
            "https://demo.tradelocker.com/backend-api/trade/accounts/{}/ordersHistory",
            account.id
        );

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
        let account = self
            .account_info
            .as_ref()
            .ok_or("no account_info for this account")?;
        let url = format!("https://demo.tradelocker.com/backend-api/trade/config");
        dotenv()?;

        let res = client
            .get(url)
            .bearer_auth(&token.access_token)
            .header("accept", "application/json")
            .header("accNum", &account.acc_num)
            .header("developer-api-key", env::var("TL_DEVELOPER_API_KEY")?)
            .send()
            .await?;

        let parsed: ConfigResponse = res.json().await?;
        self.config_data = Some(parsed.d);
        println!("Config: \n {:?}", self.config_data);
        Ok(())
    }

    pub fn zip_orders(&self) -> Result<Vec<HashMap<String, serde_json::Value>>, Box<dyn Error>> {
        let columns = &self
            .config_data
            .as_ref()
            .ok_or("config_data not loaded")?
            .orders_history_config
            .columns;
        let rows = self
            .orders_history
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

    pub fn group_position_id(
        &self,
    ) -> Result<HashMap<String, Vec<HashMap<String, serde_json::Value>>>, Box<dyn Error>> {
        let zipped_orders = self
            .zipped_orders
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

    //find tradable_intstrument_id and route_id
    pub fn find_route_id_and_instrument_id(
        &self,
        instrument_name: &str,
        target_kind: &str,
    ) -> Result<(i32, i64), Box<dyn Error>> {
        let instruments = self.instruments.as_ref().ok_or("instrument data error")?;

        let instrument = instruments
            .iter()
            .find(|inst| inst.name.starts_with(instrument_name))
            .ok_or_else(|| format!("no instrument found for {}", instrument_name))?;

        let route = instrument
            .routes
            .iter()
            .find(|r| r.kind.eq_ignore_ascii_case(target_kind))
            .ok_or_else(|| {
                format!(
                    "no {} route defined for instrument {}",
                    target_kind, instrument_name
                )
            })?;

        let instrument_route_id_tr_instrument_id = (route.id, instrument.tradable_instrument_id);

        Ok(instrument_route_id_tr_instrument_id)
    }

    //find unit value
    pub async fn find_contract_unit_value(
        &self,
        order: &OrderIntent,
        client: &Client,
    ) -> Result<Decimal, Box<dyn Error>> {
        let (route_id, instrument_id) =
            self.find_route_id_and_instrument_id(&order.instrument, "INFO")?;
        let url = format!(
            "https://demo.tradelocker.com/backend-api/trade/instruments/{instrument_id}?routeId={route_id}&locale=en"
        );
        let token = self.token.as_ref().ok_or("no token for this account")?;
        let account = self
            .account_info
            .as_ref()
            .ok_or("no account_info for this account")?;

        let res = client
            .get(url)
            .bearer_auth(&token.access_token)
            .header("accept", "application/json")
            .header("accNum", &account.acc_num)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Instruments API rejected request: HTTP {} - Body: {}",
                status, text
            )
            .into());
        }

        tokio::time::sleep(Duration::from_millis(3000)).await;

        let parsed: InstrumentDetailResponse = res.json().await?;

        // Safely unwrap the Option, throwing a descriptive error if the API returned null for lot_size
        let lot_size = parsed.d.lot_size.ok_or("API returned null for lot_size")?;

        Ok(lot_size)
    }

    //Account Balance
    pub fn find_account_balance(&self) -> Result<Decimal, Box<dyn Error>> {
        let acc_info = self
            .account_info
            .as_ref()
            .ok_or("no account_info for calculating balance")?;

        let balance: Decimal = Decimal::from_str(&acc_info.account_balance)?;

        Ok(balance)
    }

    pub async fn get_current_prices(
        &self,
        client: &Client,
        instrument_name: &str,
    ) -> Result<QuotesResponse, Box<dyn Error>> {
        let token = self.token.as_ref().ok_or("no token for this account")?;
        let account = self
            .account_info
            .as_ref()
            .ok_or("no account_info for this account")?;
        let (route_id, instrument_id) =
            self.find_route_id_and_instrument_id(instrument_name, "INFO")?;

        tokio::time::sleep(Duration::from_millis(3000)).await;

        let url = format!(
            "https://demo.tradelocker.com/backend-api/trade/quotes?routeId={}&tradableInstrumentId={}",
            route_id, instrument_id
        );

        let res = client
            .get(url)
            .bearer_auth(&token.access_token)
            .header("accept", "application/json")
            .header("accNum", &account.acc_num)
            .send()
            .await?;

        /*if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(format!("Quotes API rejected request: HTTP {} - Body: {}", status, text).into());
        }*/

        let prices: QuotesResponse = res.json().await?;

        Ok(prices)
    }

    pub async fn place_new_order(
        &self,
        order_intent: &OrderIntent,
        client: &Client,
    ) -> Result<serde_json::Value, Box<dyn Error>> {
        //self.ensure_fresh_token(client).await;

        let token = self.token.as_ref().ok_or("no token for this account")?;
        let account = self
            .account_info
            .as_ref()
            .ok_or("no account_id found for this account")?;

        let order = self.build_new_order(order_intent, client).await?;

        let url = format!(
            "https://demo.tradelocker.com/backend-api/trade/accounts/{}/orders",
            account.id
        );

        let res = client
            .post(url)
            .bearer_auth(&token.access_token)
            .json(&order)
            .header("accept", "application/json")
            .header("accNum", &account.acc_num)
            .header("developer-api-key", "")
            .send()
            .await?;

        let status = res.status();

        let response_payload: serde_json::Value = res.json().await?;

        println!("Status: {}, Order No.: {}", status, response_payload);

        Ok(response_payload)
    }

    pub async fn current_price(
        &self,
        order: &OrderIntent,
        client: &Client,
    ) -> Result<Decimal, Box<dyn Error>> {
        let instrument = order.instrument.as_ref();

        tokio::time::sleep(Duration::from_millis(3000)).await;

        let qres = self.get_current_prices(client, instrument).await?;

        // Attempt direct deserialization first. The payload 'd' is already the exact mapping we need.
        let quote = if let Ok(q) = serde_json::from_value::<CurrentPrices>(qres.d.clone()) {
            q
        } else if let Some(arr) = qres.d.as_array() {
            // Fallback just in case TradeLocker decides to wrap it in an array later
            serde_json::from_value::<CurrentPrices>(
                arr.first().cloned().ok_or("Empty quotes array")?,
            )?
        } else {
            return Err("Payload format unreadable: expected object or array".into());
        };

        let price = match order.side {
            OrderSide::Buy => quote.ap,
            OrderSide::Sell => quote.bp,
        };

        Ok(price)
    }

    pub async fn calculate_lot_size(
        &self,
        order: &OrderIntent,
        client: &Client,
    ) -> Result<Decimal, Box<dyn Error>> {
        //riskbalance * self.trade_risk_percentage()
        let current_price = self.current_price(order, client).await?;
        let pip_delta = current_price - order.stop_loss;
        let balance = self.find_account_balance()?;
        let risk = RiskCalculator::calculate_money_at_risk(order, balance);
        let contract_unit_value = self.find_contract_unit_value(order, client).await?;
        let lot_size = (risk / (pip_delta.abs() * contract_unit_value)).trunc_with_scale(2);

        println!("Balance: {}, Risk: {}", balance, risk);
        Ok(lot_size)
    }

    //Convert OrderIntent to NewOrder market (order only)
    pub async fn build_new_order(
        &self,
        order_intent: &OrderIntent,
        client: &Client,
    ) -> Result<NewOrder, Box<dyn Error>> {
        let (route_id, instrument_id) =
            self.find_route_id_and_instrument_id(&order_intent.instrument, "TRADE")?;

        let qty = self.calculate_lot_size(order_intent, client).await?;

        let new_order = NewOrder {
            qty: qty,
            route_id: route_id,
            side: order_intent.side.as_str().to_string(),
            stop_loss: order_intent.stop_loss,
            stop_loss_type: String::from("absolute"),
            take_profit: order_intent.take_profit,
            take_profit_type: String::from("absolute"),
            tradable_instrument_id: instrument_id,
            kind: String::from("market"),
            validity: String::from("IOC"),
        };
        Ok(new_order)
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
    pub async fn fetch_jwt_token(&self, client: &Client) -> Result<TokenResponse, Box<dyn Error>> {
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
    pub fn bearer_auth(&self) -> Result<String, Box<dyn Error>> {
        let bearer_auth = format!("Bearer {}", self.access_token);
        Ok(bearer_auth)
    }

    pub async fn check_token(
        &self,
        client: &Client,
    ) -> Result<Option<TokenResponse>, Box<dyn Error>> {
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
    pub instruments: Vec<InstrumentInfo>,
}

#[derive(Deserialize, Debug)]
pub struct Routes {
    pub id: i32,
    #[serde(rename = "type")]
    pub kind: String,
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
    pub d: OrderHistoryData,
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
    */
    pub rate_limits: RateLimitType,
    pub limits: Vec<ColumnDef>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RateLimit {
    pub rate_limit_type: RateLimitType,
    pub measure: RateLimitMeasure,
    pub interval_num: u32,
    pub limit: u32,
}

#[derive(Deserialize, Debug)]
pub struct Limit {
    pub limit_type: LimitType,
    pub limit: u32,
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

pub trait RiskCalculator {
    fn calculate_money_at_risk(&self, balance: Decimal) -> Decimal;
}

impl RiskCalculator for OrderIntent {
    fn calculate_money_at_risk(&self, balance: Decimal) -> Decimal {
        balance * self.setup.trade_risk_percentage()
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurrentPrices {
    pub ap: Decimal, //best ask price
    #[serde(rename = "as")]
    pub _as: Decimal, //best ask size
    pub bp: Decimal, //best bid price
    pub bs: Decimal, //best ask size
}

#[derive(Deserialize, Debug, Clone)]
pub struct QuotesResponse {
    pub d: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OrderIntent {
    //pub price: Option<Decimal>, //Change to DECIMAL
    pub instrument: String,
    pub setup: TradeSetup,
    pub side: OrderSide,
    pub stop_loss: Decimal,
    pub take_profit: Decimal,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NewOrder {
    //pub _price: Decimal, //Change to DECIMAL
    pub qty: Decimal,
    pub route_id: i32,
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

// Instrument Detail (single-instrument endpoint, not the list endpoint)
#[derive(Deserialize, Debug)]
pub struct InstrumentDetailResponse {
    pub d: InstrumentDetail,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentDetail {
    pub base_currency: Option<String>,
    pub quoting_currency: Option<String>,
    pub lot_size: Option<Decimal>,
    pub lot_step: Option<Decimal>,
    pub min_lot: Option<Decimal>,
    pub max_lot: Option<Decimal>,
    pub tick_size: Option<Vec<TickSizeTier>>,
    pub tick_cost: Option<Vec<TickCostTier>>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TickSizeTier {
    pub left_range_limit: Option<Decimal>,
    pub tick_size: Option<Decimal>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TickCostTier {
    pub left_range_limit: Option<Decimal>,
    pub tick_cost: Option<Decimal>,
}
