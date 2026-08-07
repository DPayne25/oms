/*use axum::{
    Router,
    extract::{Json, State},
    response::{Html, IntoResponse},
    routing::{get, post},
};
use oms::tradelocker::TradeSetup::DipNDot;
use oms::tradelocker::{
    OrderIntent, OrderSide::Sell, ensure_all_fresh, get_all_account_info,
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

    get_all_order_history_info(&mut accounts, &client).await;

    get_config_headers(&mut accounts, &client).await;

    //zip_all_order_history(&mut accounts).await;

    let test_pair = "EURUSD";

    for (_account_name, account) in accounts.iter_mut() {
        let quotes = account.get_current_prices(&client, test_pair).await?;
        println!("{:?}", quotes);
    }

    let stop_price = dec!(1.14078);
    let take_profit = dec!(1.12757);
    //let lot = dec!(1.22);
    let setup = DipNDot;
    let side = Sell;
    //let price = dec!(1.64538);

    let order_intent: OrderIntent = OrderIntent {
        instrument: test_pair.to_string(),
        setup: setup,
        side: side,
        stop_loss: stop_price,
        take_profit: take_profit,
    };

    for (account_name, account) in accounts.iter_mut() {
        let (route_id, instrument_id) =
            account.find_route_id_and_instrument_id(&test_pair, "TRADE")?;

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
*/

use axum::{
    Router,
    extract::{Json, State},
    response::{Html, IntoResponse},
    routing::{get, post},
};
use oms::tradelocker::{
    OrderIntent, ensure_all_fresh, get_all_account_info,
    get_config_headers, load_config,
};
use reqwest::Client;
use serde::Serialize;
use serde_json::json;
use std::{error::Error, sync::Arc};
use tokio::sync::Mutex;

pub struct AppState {
    pub accounts: Mutex<std::collections::HashMap<String, oms::tradelocker::TLAccountState>>,
    pub client: reqwest::Client,
}

#[derive(Serialize)]
struct AccountResult {
    account: String,
    ok: bool,
    message: String,
}

async fn serve_ui() -> Html<String> {
    let html = tokio::fs::read_to_string("frontend/Card Independent.html")
        .await
        .unwrap_or_else(|_| "Failed to load Card_Independent.html".to_string());
    Html(html)
}

async fn place_order_handler(
    State(state): State<Arc<AppState>>,
    Json(order_intent): Json<OrderIntent>,
) -> impl IntoResponse {
    let accounts = state.accounts.lock().await;
    let mut results = Vec::new();

    // Sequential for now — correct, isolates per-account failures, not yet
    // concurrent. JoinSet fan-out is the next upgrade, not a blocker for v1.
    for (name, account) in accounts.iter() {
        match account.place_new_order(&order_intent, &state.client).await {
            Ok(resp) => results.push(AccountResult {
                account: name.clone(),
                ok: true,
                message: resp.to_string(),
            }),
            Err(e) => results.push(AccountResult {
                account: name.clone(),
                ok: false,
                message: e.to_string(),
            }),
        }
    }

    Json(json!({ "results": results }))
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let mut accounts = load_config("config.toml")?;

    ensure_all_fresh(&mut accounts, &client).await;
    get_all_account_info(&mut accounts, &client).await;
    //get_all_order_history_info(&mut accounts, &client).await;
    get_config_headers(&mut accounts, &client).await;

    let shared_state = Arc::new(AppState {
        accounts: Mutex::new(accounts),
        client,
    });

    let app = Router::new()
        .route("/", get(serve_ui))
        .route("/order", post(place_order_handler))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("OMS listening on http://127.0.0.1:3000");
    axum::serve(listener, app).await?;

    Ok(())
}