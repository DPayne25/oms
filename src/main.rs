use axum::{
    Router,
    extract::{Json, State},
    response::{Html, IntoResponse},
    routing::{get, post},
};
use oms::tl_tradelocker::{
    OrderIntent, ensure_all_fresh, get_all_account_info,
    get_config_headers, load_config,
};
use reqwest::Client;
use serde::Serialize;
use serde_json::json;
use std::{error::Error, sync::Arc};
use tokio::sync::Mutex;

pub struct AppState {
    pub accounts: Mutex<std::collections::HashMap<String, oms::tl_tradelocker::TLAccountState>>,
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