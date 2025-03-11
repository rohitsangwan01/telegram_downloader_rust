use crate::utils::custom_result::ResultGram;
use log::error;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::OnceLock;

static APP_CONFIG: OnceLock<AppConfig> = OnceLock::new();

pub fn get_config() -> &'static AppConfig {
    APP_CONFIG
        .get()
        .expect("AppConfig has not been initialized")
}

pub fn init_config() -> ResultGram<()> {
    let config = AppConfig::from_env()?;
    APP_CONFIG
        .set(config)
        .map_err(|_| "Failed to set AppConfig")?;
    Ok(())
}

#[derive(Clone)]
pub struct AppConfig {
    pub api_id: i32,
    pub api_hash: String,
    pub bot_token: String,
    pub admins: Vec<i64>,
    pub chats: Vec<i64>,
    pub download_directory: Vec<String>,
}

impl AppConfig {
    fn from_env() -> ResultGram<Self> {
        let admins: Vec<i64> = parse_env::<String>("ADMINS")
            .ok_or("ADMINS not found")?
            .split(',')
            .skip_while(|s| s.is_empty())
            .map(|s| s.trim().parse().expect("Invalid Admin Id"))
            .collect();
        let chats: Vec<i64> = parse_env::<String>("CHATS")
            .unwrap_or("".to_string())
            .split(',')
            .skip_while(|s| s.is_empty())
            .map(|s| s.trim().parse().expect("Invalid CHATS Id"))
            .collect();

        let app_config = AppConfig {
            api_id: parse_env("TELEGRAM_API_ID").ok_or("TELEGRAM_API_ID not found")?,
            api_hash: parse_env("TELEGRAM_API_HASH").ok_or("TELEGRAM_API_HASH not found")?,
            bot_token: parse_env("BOT_TOKEN").ok_or("BOT_TOKEN not found")?,
            admins,
            chats,
            download_directory: parse_env::<String>("DOWNLOAD_DIRECTORY")
                .ok_or("DOWNLOAD_DIRECTORY not found")?
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
        };
        Ok(app_config)
    }
}

fn parse_env<T>(variable: &str) -> Option<T>
where
    T: FromStr,
    <T as FromStr>::Err: Display,
{
    dotenv::var(variable)
        .map_err(|error| error!("{error}: {variable}"))
        .ok()
        .and_then(|raw| {
            raw.parse::<T>()
                .map_err(|error| error!("{error}: {raw}"))
                .ok()
        })
}
