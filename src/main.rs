use oms::tradelocker::TradeSetup::DipNDot;
use oms::tradelocker::{OrderIntent, OrderSide::Sell, ensure_all_fresh, get_all_account_info, get_all_order_history_info, get_config_headers, load_config};
use reqwest::{Client};
//use rust_decimal::{Decimal, prelude::FromPrimitive};
use rust_decimal_macros::dec;
//use toml::value::Offset::Z;
use std::{error::Error};
use tokio;
use tokio::time::Duration;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::new();

    let mut accounts = load_config("config.toml")?;
    
    ensure_all_fresh(&mut accounts, &client).await;

    get_all_account_info(&mut accounts, &client).await;

    get_all_order_history_info(&mut accounts, &client).await;

    get_config_headers(&mut accounts, &client).await;
    
    //zip_all_order_history(&mut accounts).await;

    let test_pair = "EURUSD";
   
    for (_account_name, account) in accounts.iter_mut() {
        let quotes = account.get_current_prices(&client, test_pair).await?;
        println!("{:?}", quotes);
    }
   
   
   let stop_price  = dec!(1.14078);
    let take_profit = dec!(1.12757);
    //let lot = dec!(1.22);
    let setup = DipNDot;
    let side = Sell;
    //let price = dec!(1.64538);

    let order_intent: OrderIntent = OrderIntent{
        instrument: test_pair.to_string(),
        setup: setup,
        side: side,
        stop_loss: stop_price,
        take_profit: take_profit,
    };

    for (account_name, account) in accounts.iter_mut() {
        let (route_id, instrument_id) = account.find_route_id_and_instrument_id(&test_pair, "TRADE")?;

        println!("Account Name: {account_name}");
        println!("Pair: {test_pair} routeId: {route_id} instrumentId: {instrument_id}");

        let new_order = account.build_new_order(&order_intent, &client).await?;
    
        println!("{:?}", new_order);

        tokio::time::sleep(Duration::from_millis(3000)).await;
    }
    /*


    match account {
        Some(a) => a.place_new_order(&order_intent, &client).await?,
        None => return Err("no account with that name".into()),
    }; 
    */
    

    Ok(())
}



