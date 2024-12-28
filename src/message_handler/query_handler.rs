use grammers_client::types::CallbackQuery;

use crate::utils::{
    custom_result::ResultGram,
    download_utils::{CANCEL_DOWNLOAD, DOWNLOAD_ID_QUERY},
};

pub async fn handle_query(query: CallbackQuery) -> ResultGram<()> {
    println!("Got CallbackQuery Query {:?}", query.data());
    let mut response = "Invalid Button".to_string();

    if query.data().len() == 0 {
        query.answer().text(response).send().await?;
        return Ok(());
    }

    // First byte is query type
    let query_type = query.data()[0];

    // Handle Query Type
    if query_type == DOWNLOAD_ID_QUERY {
        response = cancel_download(query.data()).await;
    }

    query.answer().text(response).send().await?;
    return Ok(());
}

/// Handle Cancel Requests
pub async fn cancel_download(id: &[u8]) -> String {
    if id.len() < 2 {
        return "Invalid Message Id".to_string();
    }
    let download_id = id[1];
    log::info!("Cancel Download: {}", download_id);

    if let Some(cancel_token) = CANCEL_DOWNLOAD.lock().unwrap().get(&download_id) {
        cancel_token.cancel();
    }
    return "Download will be canceled shortly".to_string();
}
