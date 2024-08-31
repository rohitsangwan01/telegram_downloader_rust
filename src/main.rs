mod app_config;
mod get_bot;
mod message_handler;
mod utils;

use app_config::AppConfig;
use get_bot::get_bot;
use grammers_client::Client;
use message_handler::default_handler::handle_update;
use simple_logger::SimpleLogger;
use tokio::runtime;
use utils::{custom_result::ResultGram, helper::send_message_to_user};

fn main() -> ResultGram<()> {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init()
        .unwrap();

    runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run_bot())
}

async fn run_bot() -> ResultGram<()> {
    dotenv::dotenv().expect("please add .env file");
    let config = AppConfig::from_env().unwrap();

    log::info!("Connecting to Telegram");
    let bot: Client = get_bot(config.clone()).await?;

    send_message_to_user(bot.clone(), config.user_id, "Bot Started /help").await?;

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                log::info!("Shutting down...");
                break;
            }
            result = bot.next_update() => {
                let update = match result {
                    Ok(update) => update,
                    Err(e) => {
                        log::error!("Error getting update: {}", e);
                        break;
                    }
                };
                let bot_handler = bot.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_update(bot_handler, update).await {
                        log::error!("Error handling update: {}", e);
                    }
                });
            }
        };
    }
    Ok(())
}
