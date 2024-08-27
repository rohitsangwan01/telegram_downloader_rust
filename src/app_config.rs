use log::error;
use std::fmt::Display;
use std::str::FromStr;

use crate::utils::custom_result::ResultGram;

#[derive(Clone)]
pub struct AppConfig {
    pub api_id: i32,
    pub api_hash: String,
    pub bot_token: String,
    pub download_directory: String,
    pub user_id: i64,
}

impl AppConfig {
    pub fn from_env() -> ResultGram<Self> {
        Ok(AppConfig {
            api_id: parse_env("TELEGRAM_API_ID").ok_or("TELEGRAM_API_ID not found")?,
            api_hash: parse_env("TELEGRAM_API_HASH").ok_or("TELEGRAM_API_HASH not found")?,
            bot_token: parse_env("BOT_TOKEN").ok_or("BOT_TOKEN not found")?,
            user_id: parse_env("USER_ID").ok_or("USER_ID not found")?,
            download_directory: parse_env("DOWNLOAD_DIRECTORY")
                .ok_or("DOWNLOAD_DIRECTORY not found")?,
        })
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
