use crate::app_config::AppConfig;
use crate::utils::custom_result::ResultGram;
use crate::utils::download_utils::{delete_file, download_media_concurrent};
use crate::utils::helper::{ask_query, get_document, get_next_message};
use grammers_client::types::{media, Message};
use grammers_client::Client;
use std::collections::HashMap;
use std::fs::create_dir_all;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

lazy_static::lazy_static! {
    static ref CANCEL_DOWNLOAD: Arc<Mutex<HashMap<u8, CancellationToken>>> = Arc::new(Mutex::new(HashMap::new()));
    static ref DOWNLOAD_ID_COUNTER: Arc<Mutex<u8>> = Arc::new(Mutex::new(0));
}

/// Handle Download Requests from bot
pub async fn handle_document(bot: Client, message: Message) -> ResultGram<()> {
    let document = get_document(message.clone()).unwrap();

    let directory_result: Option<String> = get_directory(bot.clone(), message.clone()).await?;
    if directory_result.is_none() {
        return Ok(());
    }

    let use_default_file_name =
        should_download_with_default_filename(bot.clone(), message.clone(), document.clone())
            .await?;

    log::info!("DefaulFileName: {use_default_file_name}");

    let mut media_name: String = document.name().to_string();
    if !use_default_file_name {
        media_name = match get_custom_file_name(bot.clone(), message.clone()).await? {
            Some(name) => name,
            None => media_name,
        };
    }

    let directory = directory_result.unwrap();
    let dest: String = format!("{}/{}", directory, media_name);
    log::debug!("Download to : {}", dest);

    // Create download directory if it doesn't exist
    if let Err(e) = create_dir_all(&directory) {
        let error_message = format!("Failed to create download directory: {}", e.to_string());
        message.reply(error_message).await?;
        return Err(e.into());
    }

    let download_id = {
        let mut counter = DOWNLOAD_ID_COUNTER.lock().unwrap();
        if *counter == 255 {
            *counter = 0;
        }
        *counter = counter.wrapping_add(1);
        *counter
    };

    let button_id: &[u8] = &[download_id];
    log::debug!("Downloading: {:?}", button_id);

    let cancel_token = CancellationToken::new();
    {
        let mut cancel_map = CANCEL_DOWNLOAD.lock().unwrap();
        cancel_map.insert(download_id, cancel_token.clone());
    }

    let mut error: Option<String> = None;
    let start_time = std::time::Instant::now();

    if let Err(e) = download_media_concurrent(
        bot.clone(),
        dest.clone(),
        4,
        message.clone(),
        button_id,
        cancel_token.clone(),
    )
    .await
    {
        error = Some(format!("Failed To Download: {}", e.to_string()));
        log::error!("Failed {}", error.clone().unwrap());
    }

    if error.is_some() {
        message.reply(error.unwrap()).await?;
        delete_file(dest.clone()).await;
    } else {
        let download_complete_time = start_time.elapsed().as_secs();
        let mut download_time: String = format!("{download_complete_time} sec");
        if download_complete_time > 60 {
            download_time = format!("{:.1} min", download_complete_time / 60);
        }
        if download_complete_time > 3600 {
            download_time = format!("{:.1} hr", download_complete_time / 3600);
        }
        message
            .reply(format!(
                "Download Completed in {} \nStored at: {}",
                download_time, dest
            ))
            .await?;
    }

    // Remove from map
    {
        let mut cancel_map = CANCEL_DOWNLOAD.lock().unwrap();
        cancel_map.remove(&button_id[0]);
    }

    Ok(())
}

/// Handle Cancel Requests
pub async fn cancel_download(id: &[u8]) -> String {
    if id.len() == 0 {
        return "Invalid Message Id".to_string();
    }
    let download_id = id[0];
    log::info!("Cancel Download: {}", download_id);

    if let Some(cancel_token) = CANCEL_DOWNLOAD.lock().unwrap().get(&download_id) {
        cancel_token.cancel();
    }
    return "Download will be canceled shortly".to_string();
}

pub async fn get_custom_file_name(bot: Client, message: Message) -> ResultGram<Option<String>> {
    let file_name_message = message.reply("Send File Name").await?;
    let response: Message = match get_next_message(bot.clone(), message.chat().id(), 60).await {
        Some(mesage) => mesage,
        None => return Ok(None),
    };
    response.delete().await?;
    file_name_message.delete().await?;
    return Ok(Some(response.text().to_string()));
}

pub async fn should_download_with_default_filename(
    bot: Client,
    message: Message,
    document: media::Document,
) -> ResultGram<bool> {
    let options: Vec<String> = vec!["Yes".to_string(), "No".to_string()];
    let choosed_option = match ask_query(
        bot.clone(),
        message,
        format!("Download with default filename: \n{}", document.name()).as_str(),
        options.clone(),
    )
    .await?
    {
        Some(option) => option,
        None => return Ok(false),
    };
    return Ok(choosed_option == 0);
}

/// Get Directory from user if there are more then one director in env
pub async fn get_directory(bot: Client, message: Message) -> ResultGram<Option<String>> {
    let config = AppConfig::from_env().unwrap();
    let download_directories: Vec<String> = config.download_directory;

    if download_directories.len() == 1 {
        let dest: String = download_directories[0].clone();
        log::debug!("Download to : {}", dest);
        return Ok(Some(dest));
    }

    let choosed_option = ask_query(
        bot.clone(),
        message,
        "Choose a download directory:",
        download_directories.clone(),
    )
    .await?;

    if choosed_option.is_none() {
        return Ok(None);
    }

    let chosen_dir_index = choosed_option.unwrap();
    let chosen_dir = download_directories[chosen_dir_index as usize].clone();
    return Ok(Some(chosen_dir));
}
