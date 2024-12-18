use medibot::State;
use teloxide::prelude::*;
use teloxide::{
    types::{Message, ParseMode},
    Bot,
};

use crate::patient::Patient;
use crate::{ConfigParameters, HandlerResult, MyDialogue};

pub async fn start_add_patient(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        "🤒 Adding a new Patient\\. 🤕\n\nPlease provide their name:",
    )
    .parse_mode(ParseMode::MarkdownV2)
    .await?;
    dialogue.update(State::ReceivePatientName).await?;
    Ok(())
}

pub async fn receive_patient_name(
    cfg: ConfigParameters,
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(text) => {
            let patient = Patient::new(text.to_string(), msg.chat.id.to_string());
            patient.save(cfg.redis_connection).unwrap();
            bot.send_message(msg.chat.id, format!("Patient {} created.", text))
                .await?;

            dialogue.exit().await?;
        }
        None => {
            bot.send_message(msg.chat.id, "Didn't get that, please try again or /cancel.")
                .await?;
        }
    }

    Ok(())
}
