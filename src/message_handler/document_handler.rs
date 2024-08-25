use crate::app_config::AppConfig;
use crate::custom_result;
use custom_result::ResultGram;
use grammers_client::client::files::DownloadIter;
use grammers_client::types::{media, Chat, Downloadable, Media};
use grammers_client::Client;
use std::io::Write;
use std::time::Instant;
use std::{fs, fs::create_dir_all, io};

pub async fn handle_document(bot: Client, chat: Chat, document: media::Document) -> ResultGram<()> {
    let config = AppConfig::from_env().unwrap();

    let media_name: String = document.name().to_string();
    let dest = format!("{}/{}", config.download_directory, media_name);
    println!("Download to : {}", dest);

    // Create download directory if it doesn't exist
    if let Err(e) = create_dir_all(&config.download_directory) {
        let error_message = format!("Failed to create download directory: {}", e.to_string());
        bot.send_message(&chat, error_message).await?;
        return Err(e.into());
    }

    let mut download = bot.iter_download(&Downloadable::Media(Media::Document(document.clone())));
    match load(bot, chat, dest, document.clone(), &mut download).await {
        Ok(_) => {}
        Err(e) => {
            let error = format!("Failed To Download: {}", e.to_string());
            println!("Failed {error}");
        }
    }
    Ok(())
}

fn format_message(name: &str, progress: f64, speed: f64) -> String {
    return format!(
        "Downloading: {name}
        \nProgress:  {:.0}% ({:.1} MB/s)",
        progress, speed
    );
}

// Get chunks of file and save to storage
async fn load(
    bot: Client,
    chat: Chat,
    path: String,
    document: media::Document,
    download: &mut DownloadIter,
) -> ResultGram<()> {
    let mut file = match fs::File::create(path.clone()) {
        Ok(file) => file,
        Err(_) => {
            println!("Failed to create file");
            bot.send_message(&chat, format!("failed to create file {}", path))
                .await?;
            return Ok(());
        }
    };

    let total_size = document.size();
    let mut downloaded_size: i64 = 0;

    println!("Downloading: {}", total_size);

    let message = bot
        .send_message(&chat, format_message(document.name(), 0.0, 0.0))
        .await?;

    let message_id = message.id();
    let mut last_update_time = Instant::now();
    let mut last_downloaded_size = 0;
    let mut last_progress_text: String = "".to_string();

    while let Some(chunk) = download
        .next()
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    {
        downloaded_size += chunk.len() as i64;
        let progress = (downloaded_size as f64 / total_size as f64) * 100.0;

        if last_update_time.elapsed().as_secs() >= 1 {
            let bytes_downloaded_since_last_update = downloaded_size - last_downloaded_size;
            let speed_mbps = (bytes_downloaded_since_last_update as f64 / (1024.0 * 1024.0))
                / last_update_time.elapsed().as_secs_f64();

            let progress_text = format_message(document.name(), progress, speed_mbps);

            if last_progress_text != progress_text {
                bot.edit_message(&chat, message_id, progress_text.clone())
                    .await?;
                last_progress_text = progress_text;
            }
            last_update_time = Instant::now();
            last_downloaded_size = downloaded_size;
        }

        match file.write_all(&chunk) {
            Ok(_) => {}
            Err(_) => {
                bot.send_message(&chat, format!("failed to update chunk "))
                    .await?;
                return Ok(());
            }
        };
    }

    // Final update to indicate completion
    bot.edit_message(
        &chat,
        message_id,
        format!("Download Complete!, Stored at: {}", path),
    )
    .await?;

    Ok(())
}
