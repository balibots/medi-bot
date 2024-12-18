use std::error::Error;

use medibot::State;

use crate::commands::cancel;
use crate::medication::Medication;
use crate::{patient::Patient, ConfigParameters, HandlerResult, MyDialogue};

use teloxide::{
    prelude::*,
    types::{InlineKeyboardMarkup, Message, ParseMode},
    Bot,
};

pub async fn take_medicine_callback_handler(
    cfg: ConfigParameters,
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(ref patient) = q.data {
        if patient == "cancel" {
            cancel(bot, dialogue, q.regular_message().unwrap().to_owned()).await?;
        } else {
            log::info!("You chose: {patient}");

            bot.answer_callback_query(&q.id).await?;

            if let Some(message) = q.regular_message() {
                let new_keyb =
                    Medication::generate_medication_keyboard(patient, cfg.redis_connection);

                bot.edit_message_text(
                    message.chat.id,
                    message.id,
                    "Great, now the name of the medicine?",
                )
                .reply_markup(InlineKeyboardMarkup::new(new_keyb))
                .parse_mode(ParseMode::MarkdownV2)
                .await?;

                dialogue
                    .update(State::TakeMedicineFinal {
                        patient_id: patient.into(),
                    })
                    .await?;
            }
        }
    }

    Ok(())
}

pub async fn take_medicine_second_callback_handler(
    cfg: ConfigParameters,
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(ref medicine_id) = q.data {
        if medicine_id == "cancel" {
            cancel(bot, dialogue, q.regular_message().unwrap().to_owned()).await?;
        } else {
            log::info!("You chose: {medicine_id}");

            bot.answer_callback_query(&q.id).await?;

            if let Some(message) = q.regular_message() {
                let mut medicine =
                    Medication::get_by_id(medicine_id, cfg.redis_connection.clone()).unwrap();
                medicine.set_taken_now();
                medicine.save(cfg.redis_connection.clone()).unwrap();

                bot.edit_message_text(message.chat.id, message.id, "Got it, thanks!")
                    .await?;

                let medicines = Medication::get_all_by_patient_id(
                    &medicine.patient_id,
                    cfg.redis_connection.clone(),
                );

                bot.send_message(
                    message.chat.id,
                    medicines
                        .iter()
                        .map(|m| m.print_in_list() + "\n")
                        .collect::<String>(),
                )
                .await?;

                dialogue.exit().await?;
            }
        }
    }

    Ok(())
}

pub async fn take_medicine(
    cfg: ConfigParameters,
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
) -> HandlerResult {
    let con = cfg.redis_connection;

    let keyboard = Patient::generate_patient_keyboard(con.clone(), msg.chat.id.to_string(), false);

    bot.send_message(
        msg.chat.id,
        "💉*Time to take some meds\\!* 🤒 \n\n Please start by selecting the patient\\:",
    )
    .reply_markup(InlineKeyboardMarkup::new(keyboard))
    .parse_mode(ParseMode::MarkdownV2)
    .await?;

    dialogue.update(State::TakeMedicine).await?;

    Ok(())
}
