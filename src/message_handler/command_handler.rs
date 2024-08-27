use crate::custom_result;
use custom_result::ResultGram;
use grammers_client::types::{Chat, Message};
use grammers_client::Client;
use local_ip_address::local_ip;
use system_shutdown::reboot;

const START_COMMAND: &str = "/start";
const IP_COMMAND: &str = "/ip";
const INFO_COMMAND: &str = "/info";
const REBOOT_COMMAND: &str = "/reboot";
const HELP_COMMAND: &str = "/help";

pub async fn handle_command(_: Client, chat: Chat, message: Message) -> ResultGram<()> {
    let command: &str = message.text();
    let response: String = match command {
        START_COMMAND => handle_start(chat.clone()),
        IP_COMMAND => handle_ip(),
        INFO_COMMAND => handle_system_info(),
        REBOOT_COMMAND => handle_reboot(),
        _ => handle_help(chat.clone()),
    };
    message.reply(response).await?;
    return Ok(());
}

fn handle_start(chat: Chat) -> String {
    let name = chat.name();
    return format!("Welcom {}, Send me files to download", name).to_string();
}

fn handle_ip() -> String {
    let my_local_ip = local_ip().unwrap();
    return format!("Here is your ip: {}", my_local_ip).to_string();
}

fn handle_system_info() -> String {
    let fs_stats = fs2::statvfs("/").unwrap();
    let total_space = fs_stats.total_space() as f64 / 1073741824.0;
    let free_space: f64 = fs_stats.available_space() as f64 / 1073741824.0;
    return format!(
        "Here is your system info: \nTotal Space: {:.1} GB \nFree Space: {:.1} GB",
        total_space, free_space
    )
    .to_string();
}

fn handle_reboot() -> String {
    return match reboot() {
        Ok(_) => return "Restarting".to_string(),
        Err(error) => format!("Failed to shut down: {}", error),
    };
}

fn handle_help(chat: Chat) -> String {
    let name: &str = chat.name();
    return format!(
        "Hey {name}, Use these Commadns:.\n\
        {START_COMMAND} : To start the bot\n\
        {IP_COMMAND} : Get your IP\n\
        {REBOOT_COMMAND} : To reboot the machine\n\
        {INFO_COMMAND}: To get system information\n\
        {HELP_COMMAND}: To get help\n\
        \nor send files to download"
    )
    .to_string();
}
