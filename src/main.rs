use oms::tradelocker::{NewOrder, TLAccountState, ensure_all_fresh, get_all_account_info, get_all_order_history_info, get_config_headers, load_config, zip_all_order_history};
use reqwest::{Client};
use rust_decimal::{Decimal, prelude::FromPrimitive};
use rust_decimal_macros::dec;
//use toml::value::Offset::Z;
use std::{error::Error, str::FromStr};
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

    let test_pair = "AUDCAD";
    
    for (account_name, account) in accounts.iter_mut() {
        let (route_id, instrument_id) = account.find_static_instrument_info(&test_pair)?;

        println!("Account Name: {account_name}");
        println!("Pair: {test_pair} routeId: {route_id} instrumentId: {instrument_id}");
    }
    let stop_price  = dec! (0.98478);
    let target_price = dec!(0.97833);
    let lot = dec!(0.50);
    let order: NewOrder = NewOrder{
        qty: lot,
        route_id: i32::from(898485),
        side: String::from("sell"),
        stop_loss: stop_price,
        stop_loss_type: String::from("absolute"),
        take_profit: target_price,
        take_profit_type: String::from("absolute"),
        tradable_instrument_id: i32::from(4683),
        kind: String::from("market"),
        validity: String::from("IOC"),
    };

    let account = accounts.get("tradelocker-herofx-216");

    match account {
        Some(a) => a.place_new_order(&order, &client).await?,
        None => return Err("no account with that name".into()),
    };
    Ok(())
}
