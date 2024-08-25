mod app_config;
mod custom_result;
mod get_bot;
mod message_handler;

use app_config::AppConfig;
use custom_result::ResultGram;
use get_bot::get_bot;
use message_handler::default_handler::handle_update;
use std::pin::pin;

use futures_util::future::{select, Either};
use grammers_client::Client;
use simple_logger::SimpleLogger;
use tokio::{runtime, task};

fn main() -> ResultGram<()> {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init()
        .unwrap();

    runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main())
}

async fn async_main() -> ResultGram<()> {
    println!("Connecting to Telegram...");
    dotenv::dotenv().expect("failed to find .env file");

    let config = AppConfig::from_env();

    // Get Client
    let bot: Client = get_bot(config.unwrap()).await?;

    loop {
        let exit = pin!(async { tokio::signal::ctrl_c().await });
        let upd = pin!(async { bot.next_update().await });

        let update = match select(exit, upd).await {
            Either::Left(_) => break,
            Either::Right((u, _)) => u?,
        };

        let bot_handler = bot.clone();
        task::spawn(async move {
            match handle_update(bot_handler, update).await {
                Ok(_) => {}
                Err(e) => eprintln!("Error handling updates!: {e}"),
            }
        });
    }
    Ok(())
}
