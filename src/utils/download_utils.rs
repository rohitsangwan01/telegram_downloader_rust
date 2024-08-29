use crate::Client;
use grammers_client::client::files::MAX_CHUNK_SIZE;
use grammers_client::types::Media;
use grammers_client::types::Message;
use grammers_client::{button, grammers_tl_types, reply_markup, InputMessage, InvocationError};
use grammers_tl_types as tl;
use std::sync::atomic::AtomicBool;
use std::{
    io::SeekFrom,
    sync::atomic::{AtomicI64, Ordering},
    sync::Arc,
    time::Instant,
};
use tokio::sync::mpsc::unbounded_channel;
use tokio::{
    fs,
    io::{self, AsyncSeekExt, AsyncWriteExt},
};

use super::custom_result::ResultGram;

/// Modified Version of `download_media_concurrent` from library
/// Implement Cancellation of Download, and sends DownloadProgress to user
pub async fn download_media_concurrent(
    bot: Client,
    media: &Media,
    path: String,
    workers: usize,
    message: Message,
    button_id: &[u8],
    should_cancel: Arc<AtomicBool>,
) -> ResultGram<()> {
    let document = match media {
        Media::Document(document) => document,
        _ => panic!("Only Document type is supported!"),
    };
    let size = document.size();
    let location = media.to_raw_input_location().unwrap();

    // Allocate
    let mut file = fs::File::create(path.clone()).await?;
    file.set_len(size as u64).await?;
    file.seek(SeekFrom::Start(0)).await?;

    // Start workers
    let (tx, mut rx) = unbounded_channel();
    let part_index = Arc::new(tokio::sync::Mutex::new(0));
    let downloaded_size = Arc::new(AtomicI64::new(0));
    let mut tasks = vec![];

    for _ in 0..workers {
        let location = location.clone();
        let tx = tx.clone();
        let part_index = part_index.clone();
        let client = bot.clone();
        let downloaded_size = downloaded_size.clone();

        let task = tokio::task::spawn(async move {
            let mut retry_offset = None;
            let mut dc = None;
            loop {
                // Calculate file offset
                let offset: u64 = {
                    if let Some(offset) = retry_offset {
                        retry_offset = None;
                        offset
                    } else {
                        let mut i = part_index.lock().await;
                        *i += 1;
                        (MAX_CHUNK_SIZE as u64) * (*i - 1)
                    }
                };
                if (offset as i64) > size {
                    break;
                }
                // Fetch from telegram
                let request = &tl::functions::upload::GetFile {
                    precise: true,
                    cdn_supported: false,
                    location: location.clone(),
                    offset: offset as i64,
                    limit: MAX_CHUNK_SIZE,
                };
                let res = match dc {
                    None => client.invoke(request).await,
                    Some(dc) => client.invoke_in_dc(request, dc as i32).await,
                };
                match res {
                    Ok(tl::enums::upload::File::File(file)) => {
                        downloaded_size.fetch_add(file.bytes.len() as i64, Ordering::SeqCst);
                        tx.send((offset as u64, file.bytes)).unwrap();
                    }
                    Ok(tl::enums::upload::File::CdnRedirect(_)) => {
                        panic!("API returned File::CdnRedirect even though cdn_supported = false");
                    }
                    Err(InvocationError::Rpc(err)) => {
                        // File Migrate Error
                        if err.code == 303 {
                            dc = err.value;
                            retry_offset = Some(offset);
                            continue;
                        }
                        return Err(InvocationError::Rpc(err));
                    }
                    Err(e) => return Err(e),
                }
            }
            Ok::<(), InvocationError>(())
        });
        tasks.push(task);
    }
    drop(tx);

    // File write loop
    let mut last_update_time = Instant::now();
    let mut last_downloaded_size = 0;
    let mut last_progress_text: String = "".to_string();

    let mut pos = 0;
    while let Some((offset, data)) = rx.recv().await {
        if should_cancel.load(Ordering::SeqCst) {
            log::info!("Download canceled!");
            for task in tasks {
                task.abort();
            }
            delete_file(path.clone()).await;
            message.edit("Download Cancelled").await?;
            return Ok(());
        }

        if offset != pos {
            file.seek(SeekFrom::Start(offset)).await?;
        }
        file.write_all(&data).await?;
        pos = offset + data.len() as u64;

        // Update progress every 5 sec
        if last_update_time.elapsed().as_secs() >= 5 {
            let downloaded = downloaded_size.load(Ordering::SeqCst) as f64;
            let speed_mbps = ((downloaded - last_downloaded_size as f64) / (1024.0 * 1024.0))
                / last_update_time.elapsed().as_secs_f64();
            last_downloaded_size = downloaded as usize;
            last_update_time = Instant::now();

            let progress_text =
                format_message(document.name(), downloaded, size as f64, speed_mbps);

            if last_progress_text != progress_text {
                message
                    .edit(InputMessage::text(progress_text.clone()).reply_markup(
                        &reply_markup::inline(vec![vec![button::inline("Cancel", button_id)]]),
                    ))
                    .await?;
                last_progress_text = progress_text;
            }
        }
    }

    // Final update to indicate completion
    message
        .edit(format!("Download Complete! \nStored at: {}", path))
        .await?;

    // Check if all tasks finished succesfully
    for task in tasks {
        task.await?
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    }
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

pub async fn delete_file(path: String) {
    if let Err(err) = fs::remove_file(path).await {
        log::error!("Failed to delete file: {}", err);
    } else {
        log::info!("File deleted successfully")
    }
}

// Get chunks of file and save to storage
// pub async fn download_file(
//     bot: Client,
//     message: Message,
//     path: String,
//     button_id: &[u8],
//     document: media::Document,
//     should_cancel: Arc<AtomicBool>,
// ) -> ResultGram<()> {
//     let mut download = bot.iter_download(&Downloadable::Media(Media::Document(document.clone())));
//     let mut file = fs::File::create(path.clone()).await?;
//     let total_size = document.size();
//     let mut downloaded_size: i64 = 0;
//     let mut last_update_time = Instant::now();
//     let mut last_downloaded_size = 0;
//     let mut last_progress_text: String = "".to_string();

//     let progress_text = format_message(document.name(), 0.0, downloaded_size as f64, 0.0);

//     message
//         .edit(
//             InputMessage::text(progress_text).reply_markup(&reply_markup::inline(vec![vec![
//                 button::inline("Cancel", button_id),
//             ]])),
//         )
//         .await?;

//     while let Some(chunk) = download
//         .next()
//         .await
//         .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
//     {
//         if should_cancel.load(Ordering::SeqCst) {
//             log::info!("Download canceled!");
//             file.flush().await?;
//             delete_file(path.clone()).await;
//             message.edit("Download Cancelled").await?;
//             return Ok(());
//         }

//         downloaded_size += chunk.len() as i64;

//         // Send updates in 5 seconds of interval
//         if last_update_time.elapsed().as_secs() >= 5 {
//             let bytes_downloaded_since_last_update = downloaded_size - last_downloaded_size;
//             let speed_mbps = (bytes_downloaded_since_last_update as f64 / (1024.0 * 1024.0))
//                 / last_update_time.elapsed().as_secs_f64();

//             let progress_text = format_message(
//                 document.name(),
//                 downloaded_size as f64,
//                 total_size as f64,
//                 speed_mbps,
//             );

//             if last_progress_text != progress_text {
//                 message
//                     .edit(InputMessage::text(progress_text.clone()).reply_markup(
//                         &reply_markup::inline(vec![vec![button::inline("Cancel", button_id)]]),
//                     ))
//                     .await?;
//                 last_progress_text = progress_text;
//             }
//             last_update_time = Instant::now();
//             last_downloaded_size = downloaded_size;
//         }

//         file.write_all(&chunk).await?;
//     }

//     // Final update to indicate completion
//     message
//         .edit(format!("Download Complete! \nStored at: {}", path))
//         .await?;

//     Ok(())
// }
