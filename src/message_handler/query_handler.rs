use grammers_client::types::CallbackQuery;

use crate::{
    message_handler::document_handler::{cancel_download, DOWNLOAD_ID_QUERY},
    utils::custom_result::ResultGram,
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
