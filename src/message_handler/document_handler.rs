use crate::app_config::AppConfig;
use crate::utils::custom_result::ResultGram;
use crate::utils::download_utils::{delete_file, download_media_concurrent};
use grammers_client::types::{media, Chat, Message};
use grammers_client::{button, reply_markup, Client, InputMessage};
use std::collections::HashMap;
use std::fs::create_dir_all;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

lazy_static::lazy_static! {
    static ref CANCEL_DOWNLOAD: Arc<Mutex<HashMap<u8, Arc<AtomicBool>>>> = Arc::new(Mutex::new(HashMap::new()));
    static ref DOWNLOAD_ID_COUNTER: Arc<Mutex<u8>> = Arc::new(Mutex::new(0));
}

/// Handle Download Requests from bot
pub async fn handle_document(
    bot: Client,
    chat: Chat,
    message: Message,
    document: media::Document,
) -> ResultGram<()> {
    let config = AppConfig::from_env().unwrap();
    let media_name: String = document.name().to_string();
    let dest = format!("{}/{}", config.download_directory, media_name);
    log::debug!("Download to : {}", dest);

    // Create download directory if it doesn't exist
    if let Err(e) = create_dir_all(&config.download_directory) {
        let error_message = format!("Failed to create download directory: {}", e.to_string());
        bot.send_message(&chat, error_message).await?;
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

    let should_cancel = Arc::new(AtomicBool::new(false));
    {
        let mut cancel_map = CANCEL_DOWNLOAD.lock().unwrap();
        cancel_map.insert(download_id, Arc::clone(&should_cancel));
    }

    let message_reply = message
        .reply(
            InputMessage::text("Downloading..").reply_markup(&reply_markup::inline(vec![vec![
                button::inline("Cancel", button_id),
            ]])),
        )
        .await?;

    let mut error: Option<String> = None;

    if let Err(e) = download_media_concurrent(
        bot.clone(),
        &message.media().unwrap(),
        dest.clone(),
        4,
        message_reply.clone(),
        button_id,
        should_cancel,
    )
    .await
    {
        error = Some(format!("Failed To Download: {}", e.to_string()));
        log::error!("Failed {}", error.clone().unwrap());
    }

    if error.is_some() {
        message_reply.edit(error.unwrap()).await?;
        delete_file(dest.clone()).await;
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

    if let Some(should_cancel) = CANCEL_DOWNLOAD.lock().unwrap().get(&download_id) {
        should_cancel.store(true, Ordering::SeqCst);
    }
    return "Download will be canceled shortly".to_string();
}
