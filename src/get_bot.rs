use crate::app_config::AppConfig;
use crate::ResultGram;
use grammers_client::session::Session;
use grammers_client::{Client, Config, InitParams};

const BOT_SESSION_FILE: &str = "bot.session";

// Create session for this bot
pub async fn get_bot(config: AppConfig) -> ResultGram<Client> {
    let client = Client::connect(Config {
        session: Session::load_file_or_create(BOT_SESSION_FILE)?,
        api_id: config.api_id,
        api_hash: config.api_hash.to_string().clone(),
        params: InitParams {
            catch_up: false,
            ..Default::default()
        },
    })
    .await?;
    println!("Bot Connected!");

    if !client.is_authorized().await? {
        println!("Signing in...");
        client.bot_sign_in(config.bot_token.as_str()).await?;
        client.session().save_to_file(BOT_SESSION_FILE)?;
        println!("Signed in!");
    }

    Ok(client)
}
