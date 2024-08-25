use crate::custom_result;
use crate::message_handler::command_handler::handle_command;
use crate::message_handler::document_handler::handle_document;
use custom_result::ResultUpdate;
use grammers_client::types::Media::Document;
use grammers_client::types::{media, Message};
use grammers_client::{Client, Update};

pub async fn handle_update(bot: Client, update: Update) -> ResultUpdate {
    // Handle only messages sent by users
    let message = match update {
        Update::NewMessage(message) => {
            if message.outgoing() {
                return Ok(());
            }
            message
        }
        _ => return Ok(()),
    };
    let chat = message.chat();

    let document = get_document(message.clone());
    // Handle Document if available
    if document.is_some() {
        handle_document(bot, chat, document.unwrap()).await?;
        return Ok(());
    }

    // Check if a message start with /, to handle as command
    if message.text().starts_with("/") {
        handle_command(bot, chat, message).await?;
        return Ok(());
    }

    // Handle Rest of the messages
    println!("Got Message {}", message.text());
    bot.send_message(&chat, "Please Send a Message Media")
        .await?;
    Ok(())
}

/// Get only Document from the Message
fn get_document(message: Message) -> Option<media::Document> {
    match message.media() {
        Some(media) => match media {
            Document(document) => return Some(document),
            _ => return None,
        },
        None => return None,
    };
}
