use crate::app_config::AppConfig;
use crate::custom_result;
use custom_result::ResultGram;
use grammers_client::types::{media, Chat, Downloadable, Media, Message};
use grammers_client::{button, reply_markup, Client, InputMessage};
use std::collections::HashMap;
use std::fs::create_dir_all;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::{fs, io::AsyncWriteExt};

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

    if let Err(e) = download_file(
        bot.clone(),
        message_reply.clone(),
        dest.clone(),
        button_id,
        document,
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

// Get chunks of file and save to storage
async fn download_file(
    bot: Client,
    message: Message,
    path: String,
    button_id: &[u8],
    document: media::Document,
    should_cancel: Arc<AtomicBool>,
) -> ResultGram<()> {
    let mut download = bot.iter_download(&Downloadable::Media(Media::Document(document.clone())));
    let mut file = fs::File::create(path.clone()).await?;
    let total_size = document.size();
    let mut downloaded_size: i64 = 0;
    let mut last_update_time = Instant::now();
    let mut last_downloaded_size = 0;
    let mut last_progress_text: String = "".to_string();

    let progress_text = format_message(document.name(), 0.0, downloaded_size as f64, 0.0);

    message
        .edit(
            InputMessage::text(progress_text).reply_markup(&reply_markup::inline(vec![vec![
                button::inline("Cancel", button_id),
            ]])),
        )
        .await?;

    while let Some(chunk) = download
        .next()
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    {
        if should_cancel.load(Ordering::SeqCst) {
            log::info!("Download canceled!");
            file.flush().await?;
            delete_file(path.clone()).await;
            message.edit("Download Cancelled").await?;
            return Ok(());
        }

        downloaded_size += chunk.len() as i64;

        // Send updates in 5 seconds of interval
        if last_update_time.elapsed().as_secs() >= 5 {
            let bytes_downloaded_since_last_update = downloaded_size - last_downloaded_size;
            let speed_mbps = (bytes_downloaded_since_last_update as f64 / (1024.0 * 1024.0))
                / last_update_time.elapsed().as_secs_f64();

            let progress_text = format_message(
                document.name(),
                downloaded_size as f64,
                total_size as f64,
                speed_mbps,
            );

            if last_progress_text != progress_text {
                message
                    .edit(InputMessage::text(progress_text.clone()).reply_markup(
                        &reply_markup::inline(vec![vec![button::inline("Cancel", button_id)]]),
                    ))
                    .await?;
                last_progress_text = progress_text;
            }
            last_update_time = Instant::now();
            last_downloaded_size = downloaded_size;
        }

        file.write_all(&chunk).await?;
    }

    // Final update to indicate completion
    message
        .edit(format!("Download Complete! \nStored at: {}", path))
        .await?;

    Ok(())
}

/// Format the message sent to Bot
fn format_message(name: &str, downloaded_size: f64, total_size: f64, speed: f64) -> String {
    let bar_width = 10;

    let progress = if total_size > 0.0 {
        (downloaded_size / total_size) * 100.0
    } else {
        0.0
    };

    let filled_blocks: usize = (progress / 100.0 * bar_width as f64).round() as usize;
    let empty_blocks = bar_width - filled_blocks;

    let progress_bar = format!(
        "[{}] {:.2}%",
        "🟩".repeat(filled_blocks).to_string() + &"⬜".repeat(empty_blocks),
        progress
    );

    return format!(
        "Downloading {name}
        \n{:.1} MB of {:.2} MB done.\n\n{}
        \nSpeed {:.1} MB/s",
        downloaded_size / (1024.0 * 1024.0),
        total_size / (1024.0 * 1024.0),
        progress_bar,
        speed,
    );
}

async fn delete_file(path: String) {
    if let Err(err) = fs::remove_file(path).await {
        log::error!("Failed to delete file: {}", err);
    } else {
        log::info!("File deleted successfully")
    }
}
