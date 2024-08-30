use grammers_client::types::Media::Document;
use grammers_client::types::{media, CallbackQuery, Message};
use grammers_client::{button, reply_markup, Client, InputMessage, Update};
use tokio::time::{timeout, Duration};

use super::custom_result::ResultGram;

/// Get only Document from the Message
pub fn get_document(message: Message) -> Option<media::Document> {
    match message.media() {
        Some(media) => match media {
            Document(document) => return Some(document),
            _ => return None,
        },
        None => return None,
    };
}

// Ask for options, and get back result
pub async fn ask_query(
    bot: Client,
    message: Message,
    title: &str,
    options: Vec<String>,
) -> ResultGram<Option<u8>> {
    let mut buttons: Vec<Vec<button::Inline>> = Vec::new();

    for (index, option) in options.iter().enumerate() {
        buttons.push(vec![button::inline(option, [index as u8])]);
    }

    let message_reply = message
        .reply(InputMessage::text(title).reply_markup(&reply_markup::inline(buttons)))
        .await?;

    let query_result = get_callback_query_response(bot.clone(), message.chat().id(), 30).await;
    if query_result.is_none() {
        message_reply.edit("Timeout, please try again").await?;
        return Ok(None);
    }

    let query: CallbackQuery = query_result.unwrap();
    let choosen_option = query.data()[0];
    query.answer().send().await?;
    message_reply.delete().await?;
    return Ok(Some(choosen_option));
}

// Wait for Query Response
pub async fn get_callback_query_response(
    bot: Client,
    chat_id: i64,
    timeout_seconds: u64,
) -> Option<CallbackQuery> {
    let client_hadle = bot.clone();
    if let Ok(result) = timeout(Duration::from_secs(timeout_seconds), async {
        loop {
            if let Ok(update) = client_hadle.next_update().await {
                let query = match update {
                    Update::CallbackQuery(message) => Some(message),
                    _ => None,
                };
                if query.is_some() {
                    let query_message = query.unwrap().clone();
                    if query_message.chat().id() == chat_id {
                        break Some(query_message);
                    }
                }
            }
        }
    })
    .await
    {
        result
    } else {
        None
    }
}

pub async fn get_next_message(bot: Client, chat_id: i64, timeout_seconds: u64) -> Option<Message> {
    let client_hadle = bot.clone();
    if let Ok(result) = timeout(Duration::from_secs(timeout_seconds), async {
        loop {
            if let Ok(update) = client_hadle.next_update().await {
                let message: Option<Message> = match update {
                    Update::NewMessage(message) => match message.outgoing() {
                        true => None,
                        false => Some(message),
                    },
                    _ => None,
                };
                if let Some(message_data) = message {
                    if message_data.chat().id() == chat_id {
                        break Some(message_data);
                    }
                }
            }
        }
    })
    .await
    {
        result
    } else {
        None
    }
}
