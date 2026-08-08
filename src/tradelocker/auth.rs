////////////////////////////////////////////////////////////
// Auth for TradeLocker Public API
////////////////////////////////////////////////////////////
use chrono::{DateTime, Utc};
use reqwest::Client;
use rust_decimal::Decimal;
//use rust_decimal_macros;
use serde::{Deserialize, Serialize};
//use serde_json;
use std::{collections::HashMap, error::Error, str::FromStr, env};
use dotenvy::dotenv;
//use std::io::{self, Write, Read};
//use tokio::time::Duration;


////////////////////////////////////////////////////////////
// Functions
////////////////////////////////////////////////////////////


// Stores account login values into `TLAccountState` struct
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
                /*instruments: None,
                orders_history: None,
                tl_config: None,
                zipped_orders: None,
                quotes: None,*/
            },
        );
    }
    Ok(accounts)
}

// Refreshes tokens
pub async fn refresh_all_tokens(accounts: &mut HashMap<String, TLAccountState>, client: &Client) {
    // Login to accounts sourced from config.toml
    for (account_name, account) in accounts.iter_mut() {
        if let Err(e) = account.ensure_fresh_token(client).await {
            println!("{} token refresh failed: {}", account_name, e);
        }
    }
}


// List all account details
pub async fn list_all_accounts(accounts: &mut HashMap<String, TLAccountState>, client: &Client) {
    for (account_name, account) in accounts.iter_mut() {
        if let Err(e) = account.state_list_all_accounts(client).await {
            println!("{} list all accounts failed: {}", account_name, e);
        }
    }
}


////////////////////////////////////////////////////////////
// Structs & Methods
////////////////////////////////////////////////////////////


// Stores TradeLocker sign-in data
#[derive(Deserialize, Debug)]
pub struct TradeLockerConfig {
    pub tl_url: String,
    pub tl_email: String,
    pub tl_password: String,
    pub tl_server: String,
    pub tl_account_id: String,
}

// Stores login prerquisites for TradeLocker accounts
#[derive(Serialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub server: String,
}

// Stores response of fetch_jwt_token()
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expire_date: String,
}

impl TokenResponse {
    pub fn bearer_auth(&self) -> Result<String, Box<dyn Error>> {
        let bearer_auth = format!("Bearer {}", self.access_token);
        Ok(bearer_auth)
    }

    pub async fn check_token(
        &self,
        base_url: &str,
        client: &Client,
    ) -> Result<Option<TokenResponse>, Box<dyn Error>> {
        let expire_date = DateTime::parse_from_rfc3339(&self.expire_date)?;
        let now = Utc::now();
        let seconds_remaining = (expire_date.with_timezone(&Utc) - now).num_seconds();

        if seconds_remaining > 300 {
            return Ok(None);
        }

        let url = format!("{}auth/jwt/refresh", base_url);

        let payload = RefreshRequest {
            refresh_token: self.refresh_token.clone(),
        };

        let res = client.post(url).json(&payload).send().await?;
        let new_token: TokenResponse = res.json().await?;

        Ok(Some(new_token))
    }
}

#[derive(Serialize)]
pub struct RefreshRequest {
    refresh_token: String,
}


// Stores Account Info
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

// Account Details
#[derive(Deserialize, Debug)]
pub struct AllAccountsResponse {
    accounts: Vec<AccountInfo>,
}

// Manages TradeLocker state
#[derive(Debug)]
pub struct TLAccountState {
    pub config: TradeLockerConfig,
    pub token: Option<TokenResponse>,
    pub account_info: Option<AccountInfo>,
//    pub instruments: Option<Vec<InstrumentInfo>>,
//    pub orders_history: Option<Vec<Vec<serde_json::Value>>>,
//    pub config_data: Option<ConfigData>,
//    pub zipped_orders: Option<Vec<HashMap<String, serde_json::Value>>>,
//    pub quotes: Option<QuotesResponse>,
}

impl TLAccountState {

    // Fetches JWT Token
    pub async fn fetch_jwt_token(&self, login_req: &LoginRequest, client: &Client) -> Result<TokenResponse, Box<dyn Error>> {
        let url = format!("{}auth/jwt/token", self.config.tl_url); // Placed here
        let res = client.post(url).json(login_req).send().await?;
        let token_out: TokenResponse = res.json().await?;

        Ok(token_out)
    }

    // Ensures token is fresh
    pub async fn ensure_fresh_token(&mut self, client: &Client) -> Result<(), Box<dyn Error>> {
        match &self.token {
            None => {
                let login = LoginRequest {
                    email: self.config.tl_email.clone(),
                    password: self.config.tl_password.clone(),
                    server: self.config.tl_server.clone(),
                };
                let new_token = self.fetch_jwt_token(&login, client).await?;
                self.token = Some(new_token);
            }
            Some(token) => {
                if let Some(new_token) = token.check_token(&self.config.tl_url, client).await? {
                    self.token = Some(new_token);
                }
            }
        }
        Ok(())
    }

    // List all accounts
    pub async fn state_list_all_accounts(&mut self, client: &Client) -> Result<(), Box<dyn Error>> {
        let token = self.token.as_ref().ok_or("no token for this account")?;
        let url = format!("{}auth/jwt/all-accounts", self.config.tl_url);

        let res = client
            .get(url)
            .bearer_auth(&token.access_token)
            .header("accept", "application/json")
            .send()
            .await?;

        let parsed: AllAccountsResponse = res.json().await?;
        println!("{:#?}", parsed);

        let info = parsed
            .accounts
            .into_iter()
            .next()
            .ok_or("no accounts returned")?;

        self.account_info = Some(info);
        Ok(())
    }


/*
    // Get configuration
    pub async fn get_configuration(&mut self, client: &Client) -> Result<(), Box<dyn Error>> {
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
            .ok_or("orders_history not loadeed")?;

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
    }*/
}

