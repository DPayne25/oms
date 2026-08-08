////////////////////////////////////////////////////////////
// Functions: Config for Tradelocker Public API
////////////////////////////////////////////////////////////


// GET api config headers
pub async fn get_config_headers(accounts: &mut HashMap<String, TLAccountState>, client: &Client) {
    for (account_name, account) in accounts.iter_mut() {
        if let Err(e) = account.fetch_config(&client).await {
            println!("{} failed to fetch config info: {}", account_name, e);
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