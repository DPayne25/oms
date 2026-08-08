use axum::{
    Router,
    extract::{Json, State},
    response::{Html, IntoResponse},
    routing::{get, post},
};
use oms::tradelocker::TradeSetup::DipNDot;
use oms::tradelocker::{
    OrderIntent, OrderSide::Sell, ensure_all_fresh, get_all_account_info, get_config_headers,
    get_all_order_history_info, get_config_headers, load_config,
};
use reqwest::Client;
use rust_decimal_macros::dec;
use serde_json::json;
use std::{error::Error, sync::Arc};
use tokio::{sync::Mutex, time::Duration};

pub struct AppState {
    pub accounts: Mutex<std::collections::HashMap<String, oms::tradelocker::TLAccountState>>,
    pub client: reqwest::Client,
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::new();

    let mut accounts = load_config("config.toml")?;

    ensure_all_fresh(&mut accounts, &client).await;

    get_all_account_info(&mut accounts, &client).await;

    //get_all_order_history_info(&mut accounts, &client).await;

    get_config_headers(&mut accounts, &client).await;


  
    /*


    match account {
        Some(a) => a.place_new_order(&order_intent, &client).await?,
        None => return Err("no account with that name".into()),
    };
    */

    Ok(())
}
