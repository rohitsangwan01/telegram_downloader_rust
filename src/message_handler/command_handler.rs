use crate::utils::custom_result::ResultGram;
use crate::utils::helper::{get_custom_file_name, get_directory};
use grammers_client::Client;
use grammers_client::types::{Chat, Message};
use local_ip_address::local_ip;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

const START_COMMAND: &str = "/start";
const IP_COMMAND: &str = "/ip";
const INFO_COMMAND: &str = "/info";
const REBOOT_COMMAND: &str = "/reboot";
const HELP_COMMAND: &str = "/help";
const GDOWN_COMMAND: &str = "/gdown";
const SPEED_TEST_COMMAND: &str = "/speed";

const ALLOWED_COMMANDS_FOR_NON_ADMIN: [&str; 3] = [START_COMMAND, HELP_COMMAND, SPEED_TEST_COMMAND];

fn extract_command(input: &str) -> &str {
    input
        .split(|c: char| c == '@' || c == ' ')
        .next()
        .unwrap_or("")
}

pub async fn handle_command(
    bot: Client,
    chat: Chat,
    message: Message,
    is_allowed: bool,
) -> ResultGram<()> {
    let command: &str = message.text();
    let command_only = extract_command(command);
    if !is_allowed && !ALLOWED_COMMANDS_FOR_NON_ADMIN.contains(&command_only) {
        message.reply("Not Allowed to use this command").await?;
        return Ok(());
    }

    if command.contains(SPEED_TEST_COMMAND) {
        handle_speedtest(message.clone()).await?;
        return Ok(());
    } else if command.contains(GDOWN_COMMAND) {
        download_gdrive(bot.clone(), message.clone()).await?;
        return Ok(());
    }

    // Handle Text based commands
    let response: String = match command {
        START_COMMAND => handle_start(chat.clone()),
        IP_COMMAND => handle_ip(),
        INFO_COMMAND => handle_system_info(),
        REBOOT_COMMAND => handle_reboot(),
        _ => handle_help(chat.clone(), is_allowed),
    };
    message.reply(response).await?;
    return Ok(());
}

fn handle_help(chat: Chat, is_allowed: bool) -> String {
    let name: &str = chat.name().unwrap_or("N/A");
    let mut commands = vec![
        format!("{START_COMMAND}: To start the bot"),
        format!("{HELP_COMMAND}: To get help"),
        format!("{SPEED_TEST_COMMAND}: Get network speed"),
    ];

    if is_allowed {
        commands.extend(vec![
            format!("{IP_COMMAND}: Get your IP"),
            format!("{INFO_COMMAND}: To get system information"),
            format!("{GDOWN_COMMAND}: To download gdrive files"),
            format!("{REBOOT_COMMAND}: To reboot the machine"),
        ]);
    }

    format!(
        "Hey {name}, Use these commands:\n{}\n\
        \nor send files to download",
        commands.join("\n")
    )
}

fn handle_start(chat: Chat) -> String {
    let name = chat.name().unwrap_or("N/A");
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

pub async fn handle_speedtest(message: Message) -> ResultGram<()> {
    let reply_message = message.reply("Starting Speedtest, please wait...").await?;

    let mut command = Command::new("speedtest")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut response = "".to_string();
    let mut last_message = "".to_string();
    if let Some(stdout) = command.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                response = format!("{}\n{line}", response.clone()).trim().to_string();
                if response.trim().is_empty() {
                    continue;
                }
                if response != last_message {
                    println!("{}", response.clone());
                    reply_message.edit(response.clone()).await?;
                    last_message = response.clone();
                }
            }
        }
    }

    let _ = command.wait()?;
    reply_message.edit(format_internet_speed(&response)).await?;
    return Ok(());
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

fn format_internet_speed(input: &str) -> String {
    let mut lines = input
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty()) // Remove empty lines
        .collect::<Vec<_>>();

    // Remove unnecessary lines
    if let Some(pos) = lines
        .iter()
        .position(|&line| line.starts_with("Speedtest by Ookla"))
    {
        lines.remove(pos);
    }
    if let Some(pos) = lines.iter().position(|&line| line.starts_with("Server:")) {
        lines.remove(pos);
    }
    if let Some(pos) = lines.iter().position(|&line| line.starts_with("ISP:")) {
        lines.remove(pos);
    }

    // Reorder lines for clarity
    let mut result = Vec::new();
    let identifiers = [
        "Idle Latency:",
        "Download:",
        "Upload:",
        "Packet Loss:",
        "Result URL:",
    ];
    for line in lines {
        for identifier in &identifiers {
            if line.starts_with(identifier) {
                let value = line.replace(identifier, "").trim().to_string();
                result.push(format!("{identifier} {value}"));
                break;
            }
        }
    }

    result.join("\n")
}
