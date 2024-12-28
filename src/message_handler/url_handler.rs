use crate::utils::custom_result::ResultGram;
use crate::utils::download_utils::{format_message, should_download_with_default_filename};
use crate::utils::helper::{get_custom_file_name, get_directory};
use grammers_client::types::Message;
use grammers_client::Client;
use std::fs::create_dir_all;
use std::fs::File;
use std::io::Write;
use std::time::Instant;
use url::Url;

pub async fn handle_url(bot: Client, message: Message) -> ResultGram<()> {
    let file_url = message.text().trim().to_string();
    let parsed_url = Url::parse(file_url.as_str())?;
    log::info!("File Download: {file_url}");

    let directory_result: Option<String> = get_directory(bot.clone(), message.clone()).await?;
    if directory_result.is_none() {
        return Ok(());
    }

    let file_name = parsed_url
        .path_segments()
        .and_then(|segments| segments.last())
        .filter(|&name| !name.is_empty());

    let media_name: Option<String>;
    if file_name.is_some() {
        let use_default_file_name = should_download_with_default_filename(
            bot.clone(),
            message.clone(),
            file_name.unwrap().to_string(),
        )
        .await?;

        if !use_default_file_name {
            media_name = get_custom_file_name(bot.clone(), message.clone()).await?;
        } else {
            media_name = Some(file_name.unwrap().to_string());
        }
    } else {
        media_name = get_custom_file_name(bot.clone(), message.clone()).await?;
    }

    if media_name.is_none() {
        return Ok(());
    }

    let media_name: String = media_name.unwrap();

    let directory = directory_result.unwrap();
    let dest: String = format!("{}/{}", directory, media_name);
    log::debug!("Download to : {}", dest);

    // Create download directory if it doesn't exist
    if let Err(e) = create_dir_all(&directory) {
        let error_message = format!("Failed to create download directory: {}", e.to_string());
        message.reply(error_message).await?;
        return Err(e.into());
    }

    let reply_message = message.reply("Starting download").await?;

    // Create HTTP client
    let client = reqwest::Client::new();

    // Send HEAD request to get the content length
    let response = client.head(file_url.clone()).send().await?;
    let total_size = response
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|val| val.to_str().ok())
        .and_then(|val| val.parse::<u64>().ok())
        .unwrap_or(0);

    // Send GET request
    let mut response = client.get(file_url).send().await?;
    response.error_for_status_ref()?;

    // Create file
    let mut file = File::create(dest.clone())?;
    let mut last_update_time = Instant::now();
    let mut last_downloaded_size = 0;
    let mut last_progress_text: String = "".to_string();
    let start_time = std::time::Instant::now();
    let mut downloaded: f64 = 0.0;

    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk)?;
        downloaded += chunk.len() as f64;
        // Update progress every 5 sec
        if last_update_time.elapsed().as_secs() >= 5 {
            let speed_mbps = ((downloaded - last_downloaded_size as f64) / (1024.0 * 1024.0))
                / last_update_time.elapsed().as_secs_f64();
            last_downloaded_size = downloaded as usize;
            last_update_time = Instant::now();

            let progress_text = format_message(
                media_name.as_str(),
                downloaded,
                total_size as f64,
                speed_mbps,
            );

            if last_progress_text != progress_text {
                reply_message.edit(progress_text.clone()).await?;
                last_progress_text = progress_text;
            }
        }
    }

    reply_message.delete().await?;
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
    return Ok(());
}
