use std::error::Error;

use crate::commands::cancel;
use crate::frequency::Frequency;
use crate::medication::Medication;
use crate::patient::Patient;
use crate::{ConfigParameters, HandlerResult, MyDialogue, State};
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardMarkup, Message, ParseMode};
use teloxide::Bot;

const ERROR_NO_TEXT: &str = "Sorry, couldn't understand that - please send a text message.";

const START_FLOW_TEXT: &str =
    "💊 *Adding a new medication plan\\.* 💊 \n\n Please start by selecting the patient\\:";

pub async fn start_add_medication(
    cfg: ConfigParameters,
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
) -> HandlerResult {
    let con = cfg.redis_connection;

    let keyboard = Patient::generate_patient_keyboard(con.clone(), msg.chat.id.to_string(), true);

    bot.send_message(msg.chat.id, START_FLOW_TEXT)
        .reply_markup(InlineKeyboardMarkup::new(keyboard))
        .parse_mode(ParseMode::MarkdownV2)
        .await?;

    dialogue.update(State::ReceiveName).await?;

    Ok(())
}

pub async fn receive_name_callback_handler(
    cfg: ConfigParameters,
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(ref patient_id) = q.data {
        if patient_id == "cancel" {
            cancel(bot, dialogue, q.regular_message().unwrap().to_owned()).await?;
        } else if patient_id == "add_new" {
            let message = q.regular_message().unwrap();
            bot.edit_message_text(
                message.chat.id,
                message.id,
                "Ok, tell me what's the new patient's name.",
            )
            .await?;

            dialogue.update(State::ReceiveName).await?;
        } else {
            log::info!("You chose: {patient_id}");

            bot.answer_callback_query(&q.id).await?;

            let patient = Patient::get_by_id(patient_id, cfg.redis_connection).unwrap();

            if let Some(message) = q.regular_message() {
                bot.edit_message_text(
                    message.chat.id,
                    message.id,
                    format!(
                        "Great. Now what's the name of the medicine {} is going to be taking?",
                        patient.name
                    ),
                )
                .await?;
                dialogue
                    .update(State::ReceiveMedicine {
                        patient_id: patient_id.into(),
                    })
                    .await?;
            }
        }
    }

    Ok(())
}

pub async fn receive_name(
    cfg: ConfigParameters,
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(text) => {
            let patient = Patient::new(text.to_string(), msg.chat.id.to_string());
            patient.save(cfg.redis_connection).unwrap();

            bot.send_message(
                msg.chat.id,
                format!(
                    "Great. Now what's the name of the medicine {} is going to be taking?",
                    patient.name
                ),
            )
            .await?;

            dialogue
                .update(State::ReceiveMedicine {
                    patient_id: patient.id,
                })
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
    patient_id: String,
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(text) => {
            bot.send_message(msg.chat.id, "And what's the dosage?")
                .await?;
            dialogue
                .update(State::ReceiveDosage {
                    patient_id,
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
    (patient_id, medicine): (String, String),
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(dosage) => {
            bot.send_message(msg.chat.id, "Finally, what's the medication frequency? \\(e\\.g\\., `every 6 hours`, or `3 times a day`\\)")
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
            dialogue
                .update(State::ReceiveFrequency {
                    patient_id,
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
    (patient_id, medicine, dosage): (String, String, String),
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(frequency_str) => {
            if let Some(frequency) = Frequency::parse(frequency_str) {
                let mut medication = Medication::new(
                    patient_id,
                    medicine,
                    dosage,
                    frequency,
                    dialogue.chat_id().to_string(),
                );

                medication.save(cfg.redis_connection).unwrap();

                let report = format!(
                    "
Got it\\. Adding a new plan of `{}` to `{}`'s plan: `{}`, `{}`\\.

When giving the first dose, run /take\\.
",
                    medication.medicine,
                    medication.patient_name.clone().unwrap(),
                    medication.dosage,
                    frequency_str,
                );

                bot.send_message(msg.chat.id, report)
                    .parse_mode(ParseMode::MarkdownV2)
                    .await?;

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
