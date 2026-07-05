use oms::tradelocker::{load_config, ensure_all_fresh};
use reqwest::{Client};
use std::{error::Error};
use tokio;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::new();

    let mut accounts = load_config("config.toml")?;
    
    ensure_all_fresh(&mut accounts, &client).await;


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
    for (account_name, account) in accounts.iter_mut() {
       if let Err(e) = account.fetch_order_history(&client).await {
            println!("{} failed to fetch order history info: {}", account_name, e);
            if let Some(source) = e.source() {
                println!(" caused by: {}", source);
            }
        }
    }
    for (account_name, account) in accounts.iter_mut() {
        if let Err(e) = account.fetch_config(&client).await {
            println!("{} failed to fetch config info: {}", account_name, e);
            if let Some(source) = e.source() {
                println!(" caused by: {}", source);
            }
        }
    }
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
    Ok(())
}
