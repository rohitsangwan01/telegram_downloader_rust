use grammers_client::{types::Message, Client};

use crate::utils::custom_result::ResultGram;

pub async fn handle_url(_: Client, message: Message) -> ResultGram<()> {
    // Check if its a gdrive url
    message.reply("Url downloading not supported yet").await?;
    return Ok(());
}
