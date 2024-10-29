use crate::{Command, HandlerResult, MyDialogue};
use teloxide::{prelude::*, types::Message, utils::command::BotCommands, Bot};

pub async fn start(bot: Bot, _dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "helllooooooo whatsup")
        .await?;
    Ok(())
}

pub async fn help(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

pub async fn cancel(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Cancelling the current operation.")
        .await?;
    dialogue.exit().await?;
    Ok(())
}
