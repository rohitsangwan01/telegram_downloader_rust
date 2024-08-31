use crate::message_handler::command_handler::handle_command;
use crate::message_handler::document_handler::handle_document;
use crate::message_handler::query_handler::handle_query;
use crate::utils::custom_result::ResultUpdate;
use crate::utils::helper::get_document;
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
        Update::CallbackQuery(message) => {
            handle_query(message).await?;
            return Ok(());
        }
        _ => return Ok(()),
    };
    let chat = message.chat();

    // Handle Document if available
    if get_document(message.clone()).is_some() {
        handle_document(bot, message).await?;
        return Ok(());
    }

    // Check if a message start with /, to handle as command
    if message.text().starts_with("/") {
        handle_command(bot, chat, message).await?;
        return Ok(());
    }

    // Handle Rest of the messages
    log::debug!("Got Message {}", message.text());
    bot.send_message(&chat, "Please Send a Message Media /help")
        .await?;
    Ok(())
}
