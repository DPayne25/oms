use std::{error::Error, collections::HashMap};
use oms::{LoginRequest, TokenResponse, load_config};
fn main() {
    println!("Hello, world!");
}

async fn add_to_main()-> Result<(), Box<dyn Error>> {
    let accounts = load_config("config.toml")?;
    let mut account_tokens: HashMap<String, TokenResponse> = HashMap::new();

    for (account_name, config) in &accounts {

        let  login = LoginRequest {
            email: config.tl_email.clone(),
            password: config.tl_password.clone(),
            server: config.tl_server.clone(),
        };

        let token = login.tl_login().await;

        match token {
            Ok(t) => {
                account_tokens.insert(account_name.clone(), t);
                println!("{} login successful", account_name)
                                
            }
            Err(e) => {
                println!("{} failed to login: {}", account_name, e);
            }
        }

    }

    Ok(())

}