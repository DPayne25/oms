use oms::tradelocker::{ensure_all_fresh, get_all_account_info, get_all_order_history_info, get_config_headers, load_config, zip_all_order_history};
use reqwest::{Client};
//use toml::value::Offset::Z;
use std::{error::Error};
use tokio;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::new();

    let mut accounts = load_config("config.toml")?;
    
    ensure_all_fresh(&mut accounts, &client).await;

    get_all_account_info(&mut accounts, &client).await;

    get_all_order_history_info(&mut accounts, &client).await;

    get_config_headers(&mut accounts, &client).await;
    
    zip_all_order_history(&mut accounts).await;

    Ok(())
}
