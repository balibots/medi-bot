use crate::commands::cancel;
use medibot::State;
use std::error::Error;
use teloxide::prelude::*;
use teloxide::types::InlineKeyboardMarkup;
use teloxide::{
    types::{Message, ParseMode},
    Bot,
};

use crate::patient::Patient;
use crate::{ConfigParameters, HandlerResult, MyDialogue};

pub async fn start_share_patient(
    cfg: ConfigParameters,
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
) -> HandlerResult {
    let con = cfg.redis_connection;
    let keyboard = Patient::generate_patient_keyboard(con.clone(), msg.chat.id.to_string(), false);

    bot.send_message(
        msg.chat.id,
        "Please select the patient you'd like to share access to with another Telegram user:",
    )
    .reply_markup(InlineKeyboardMarkup::new(keyboard))
    .parse_mode(ParseMode::MarkdownV2)
    .await?;

    dialogue.update(State::StartSharePatient).await?;

    Ok(())
}

pub async fn take_patient_for_sharing_callback_handler(
    cfg: ConfigParameters,
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(ref patient_id) = q.data {
        if patient_id == "cancel" {
            cancel(bot, dialogue, q.regular_message().unwrap().to_owned()).await?;
        } else {
            log::info!("You chose: {patient_id}");

            bot.answer_callback_query(&q.id).await?;

            let patient = Patient::get_by_id(patient_id, cfg.redis_connection).unwrap();

            if let Some(message) = q.regular_message() {
                bot.edit_message_text(
                message.chat.id,
                message.id,
                format!(
                    "Great. Now please enter the Telegram User ID you'll be sharing patient {} with.",
                    patient.name
                ),
            )
            .await?;
                dialogue
                    .update(State::ReceiveTelegramUserForSharePatient {
                        patient_id: patient_id.into(),
                    })
                    .await?;
            }
        }
    }
    Ok(())
}

pub async fn receive_telegram_user_name(
    cfg: ConfigParameters,
    bot: Bot,
    dialogue: MyDialogue,
    patient_id: String,
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(text) => {
            let con = cfg.redis_connection;

            let patient = Patient::get_by_id(&patient_id, con.clone())
                .expect("Error getting patient for sharing");

            Patient::share(&patient, text, con.clone())?;

            bot.send_message(msg.chat.id, "Patient shared.").await?;

            dialogue.exit().await?;
        }
        None => {
            bot.send_message(msg.chat.id, "Didn't get that, please try again or /cancel.")
                .await?;
        }
    }

    Ok(())
}
