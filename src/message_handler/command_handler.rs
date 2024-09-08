use std::process::Command;

use grammers_client::types::{Chat, Message};
use grammers_client::Client;
use local_ip_address::local_ip;

use crate::utils::custom_result::ResultGram;
use crate::utils::helper::{get_custom_file_name, get_directory};

const START_COMMAND: &str = "/start";
const IP_COMMAND: &str = "/ip";
const INFO_COMMAND: &str = "/info";
const REBOOT_COMMAND: &str = "/reboot";
const HELP_COMMAND: &str = "/help";
const GDOWN_COMMAND: &str = "/gdown";

pub async fn handle_command(bot: Client, chat: Chat, message: Message) -> ResultGram<()> {
    let command: &str = message.text();

    if command.contains(GDOWN_COMMAND) {
        download_gdrive(bot.clone(), message.clone()).await?;
        return Ok(());
    }

    // Handle Text based commands
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

fn handle_help(chat: Chat) -> String {
    let name: &str = chat.name();
    return format!(
        "Hey {name}, Use these Commadns:.\n\
        {START_COMMAND} : To start the bot\n\
        {IP_COMMAND} : Get your IP\n\
        {REBOOT_COMMAND} : To reboot the machine\n\
        {INFO_COMMAND}: To get system information\n\
        {HELP_COMMAND}: To get help\n\
        {GDOWN_COMMAND}: To download gdrive files\n\
        \nor send files to download"
    )
    .to_string();
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
    if cfg!(target_os = "linux") {
        let output = Command::new("sudo")
            .arg("reboot")
            .output()
            .expect("failed to execute process");
        return String::from_utf8_lossy(&output.stdout).to_string();
    }
    return "Not supported yet".to_string();
}

/// If gdown installed in system
pub async fn download_gdrive(bot: Client, message: Message) -> ResultGram<()> {
    let gdrive_id = message.text().replace(GDOWN_COMMAND, "").trim().to_string();

    log::info!("Grdive Download: {gdrive_id}");

    if gdrive_id.is_empty() {
        message
            .reply("Please send a valid gdrive link or id")
            .await?;
        return Ok(());
    }

    let directory_result: Option<String> = get_directory(bot.clone(), message.clone()).await?;
    if directory_result.is_none() {
        return Ok(());
    }

    let media_name = match get_custom_file_name(bot.clone(), message.clone()).await? {
        Some(name) => name,
        None => return Ok(()),
    };
    let path = format!("{}/{media_name}", directory_result.unwrap());

    let reply_message = message.reply("Starting GoogleDrive download").await?;

    // Run a command and capture the output
    let output = Command::new("gdown")
        .arg(gdrive_id)
        .arg("-O")
        .arg(path)
        .output()
        .expect("Failed to execute command");

    // Convert the output to a String
    let stderr = String::from_utf8_lossy(&output.stderr);

    reply_message.delete().await?;

    // Print the command's output
    if !stderr.is_empty() && !stderr.contains("█") {
        eprintln!("Error: {}", stderr);
        message.reply(format!("Process Failed: {}", stderr)).await?;
    } else {
        message.reply(format!("Process Completed")).await?;
    }

    return Ok(());
}
