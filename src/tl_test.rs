use reqwest::Client;
use std::{error::Error, };
//use tokio::{sync::Mutex};
mod tradelocker;
use tradelocker::auth::{load_config, refresh_all_tokens, list_all_accounts};


/*pub struct AppState {
    pub accounts: Mutex<std::collections::HashMap<String, oms::tradelocker::TLAccountState>>,
    pub client: reqwest::Client,
}*/

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::new();

    let mut accounts = load_config("config.toml")?;

    refresh_all_tokens(&mut accounts, &client).await;

    list_all_accounts(&mut accounts, &client).await;
 /*
    get_all_account_info(&mut accounts, &client).await;

    //get_all_order_history_info(&mut accounts, &client).await;

    get_config_headers(&mut accounts, &client).await;


  



    match account {
        Some(a) => a.place_new_order(&order_intent, &client).await?,
        None => return Err("no account with that name".into()),
    };
    */

    Ok(())
}
