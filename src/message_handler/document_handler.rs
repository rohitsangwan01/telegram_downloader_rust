use crate::utils::custom_result::ResultGram;
use crate::utils::download_utils::{
    delete_file, download_media_concurrent, should_download_with_default_filename, CANCEL_DOWNLOAD,
    DOWNLOAD_ID_COUNTER, DOWNLOAD_ID_QUERY,
};
use crate::utils::helper::{get_custom_file_name, get_directory, get_document};
use grammers_client::types::Message;
use grammers_client::Client;
use std::fs::create_dir_all;
use tokio_util::sync::CancellationToken;

/// Handle Download Requests from bot
pub async fn handle_document(bot: Client, message: Message) -> ResultGram<()> {
    let document = get_document(message.clone()).unwrap();

    let directory_result: Option<String> = get_directory(bot.clone(), message.clone()).await?;
    if directory_result.is_none() {
        return Ok(());
    }

    let use_default_file_name = should_download_with_default_filename(
        bot.clone(),
        message.clone(),
        document.clone().name().to_string(),
    )
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

    let button_id: &[u8] = &[DOWNLOAD_ID_QUERY, download_id];
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
