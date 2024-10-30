use crate::frequency::Frequency;
use crate::medication::Medication;
use crate::{ConfigParameters, HandlerResult, MyDialogue, State};
use teloxide::prelude::*;
use teloxide::types::{Message, ParseMode};
use teloxide::Bot;

const ERROR_NO_TEXT: &str = "Sorry, couldn't understand that - please send a text message.";

pub async fn start_add_medication(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        "💊 *Adding a new medication plan\\.* 💊 \n\nQuick recoveries! 🤞\\. Please start by giving me the patient name\\:",
    )
    .parse_mode(ParseMode::MarkdownV2)
    .await?;
    dialogue.update(State::ReceiveName).await?;
    Ok(())
}

pub async fn receive_name(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    match msg.text() {
        Some(text) => {
            bot.send_message(msg.chat.id, "Great, now the name of the medicine?")
                .await?;
            dialogue
                .update(State::ReceiveMedicine { name: text.into() })
                .await?;
        }
        None => {
            bot.send_message(msg.chat.id, ERROR_NO_TEXT).await?;
        }
    }

    Ok(())
}

pub async fn receive_medicine(
    bot: Bot,
    dialogue: MyDialogue,
    name: String,
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(text) => {
            bot.send_message(msg.chat.id, "What's the dosage?").await?;
            dialogue
                .update(State::ReceiveDosage {
                    name,
                    medicine: text.into(),
                })
                .await?;
        }
        None => {
            bot.send_message(msg.chat.id, ERROR_NO_TEXT).await?;
        }
    }

    Ok(())
}

pub async fn receive_dosage(
    bot: Bot,
    dialogue: MyDialogue,
    (name, medicine): (String, String),
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(dosage) => {
            bot.send_message(msg.chat.id, "Finally, what's the medication frequency? \\(e\\.g\\., `every 6 hours`, or `3 times a day`\\)")
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
            dialogue
                .update(State::ReceiveFrequency {
                    name,
                    medicine,
                    dosage: dosage.into(),
                })
                .await?;
        }
        None => {
            bot.send_message(msg.chat.id, ERROR_NO_TEXT).await?;
        }
    }

    Ok(())
}

pub async fn receive_frequency(
    cfg: ConfigParameters,
    bot: Bot,
    dialogue: MyDialogue,
    (name, medicine, dosage): (String, String, String),
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(frequency_str) => {
            if let Some(frequency) = Frequency::parse(frequency_str) {
                let report = format!(
                    "Got it\\. Adding a new plan of `{}` to `{}`'s plan: `{}`, `{}`\\.",
                    medicine, name, dosage, frequency_str
                );
                bot.send_message(msg.chat.id, report)
                    .parse_mode(ParseMode::MarkdownV2)
                    .await?;

                let medication = Medication::new(
                    name,
                    medicine,
                    dosage,
                    frequency,
                    dialogue.chat_id().to_string(),
                );

                medication.save(cfg.redis_connection).unwrap();

                dialogue.exit().await?;
            } else {
                bot.send_message(msg.chat.id, "Didn't quite get that. Can you try again? (ie, every 6 hours, 3 times a day,...)").await?;
            }
        }
        None => {
            bot.send_message(msg.chat.id, ERROR_NO_TEXT).await?;
        }
    }

    Ok(())
}

// TODO temp, just so we dont go through the flow for now
pub async fn test_add(cfg: ConfigParameters, bot: Bot, msg: Message) -> HandlerResult {
    let name = "xavi".to_string();
    let medicine = "nurofen".to_string();
    let dosage = "5ml".to_string();
    let f = Frequency::parse("every 3 hours").unwrap();
    let medication = Medication::new(name, medicine, dosage, f, "123".to_string());

    medication.save(cfg.redis_connection).unwrap();

    bot.send_message(msg.chat.id, "dev: tested the add / save function")
        .await?;
    Ok(())
}
