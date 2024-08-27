mod app_config;
mod custom_result;
mod get_bot;
mod message_handler;
mod utils;

use app_config::AppConfig;
use custom_result::ResultGram;
use get_bot::get_bot;
use message_handler::default_handler::handle_update;
use std::pin::pin;

use futures_util::future::{select, Either};
use grammers_client::{session::PackedType, types::PackedChat, Client};
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
    log::info!("Connecting to Telegram");
    dotenv::dotenv().expect("failed to find .env file");

    let config = AppConfig::from_env().unwrap();

    // Get Client
    let bot: Client = get_bot(config.clone()).await?;
    let user_id = config.user_id;
    log::info!("Send message to: {}", user_id);

    let packed_chat = PackedChat {
        ty: PackedType::User,
        id: user_id,
        access_hash: Some(0),
    };
    let chat = bot.unpack_chat(packed_chat).await?;
    bot.send_message(&chat, "Bot Started").await?;

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
                Err(e) => log::error!("Error handling updates!: {e}"),
            }
        });
    }
    Ok(())
}
