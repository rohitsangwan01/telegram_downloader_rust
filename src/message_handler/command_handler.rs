use crate::custom_result;
use custom_result::ResultGram;
use grammers_client::types::{Chat, Message};
use grammers_client::Client;

pub async fn handle_command(bot: Client, chat: Chat, message: Message) -> ResultGram<()> {
    let command: &str = message.text();
    let response: String = match command {
        "/start" => handle_start(chat.clone()),
        _ => handle_help(chat.clone()),
    };

    bot.send_message(&chat, response).await?;
    return Ok(());
}

fn handle_start(chat: Chat) -> String {
    let name = chat.name();
    return format!("Welcom {}, Send me files to download", name).to_string();
}

fn handle_help(chat: Chat) -> String {
    let name = chat.name();
    return format!("Hey {name} \nsend /start to start the bot \nsend files to download")
        .to_string();
}
