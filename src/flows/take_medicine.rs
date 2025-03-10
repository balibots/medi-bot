use std::error::Error;

use medibot::State;

use crate::commands::cancel_with_edit;
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
    if let Some(ref patient_id) = q.data {
        if patient_id == "cancel" {
            cancel_with_edit(bot, dialogue, q.regular_message().unwrap().to_owned()).await?;
        } else {
            log::info!("You chose: {patient_id}");
            let con = cfg.redis_connection;
            let message = q.regular_message().unwrap();

            bot.answer_callback_query(&q.id).await?;

            let medication = Medication::get_all_by_patient_id(patient_id, con.clone());
            let patient = Patient::get_by_id(patient_id, con.clone()).unwrap();

            if medication.len() == 0 {
                bot.edit_message_text(
                    message.chat.id,
                    message.id,
                    format!("Sorry you haven't added any medication plans for {} yet, try /addmedication.", patient.name)
                )
                .await?;

                dialogue.exit().await?;
            } else {
                let new_keyb = Medication::generate_medication_keyboard(patient_id, con.clone());

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
                        patient_id: patient_id.into(),
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
    patient_id: String,
    q: CallbackQuery,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(ref medicine_id) = q.data {
        if medicine_id == "cancel" {
            cancel_with_edit(bot, dialogue, q.regular_message().unwrap().to_owned()).await?;
        } else {
            log::info!("You chose: {medicine_id}");

            bot.answer_callback_query(&q.id).await?;

            if let Some(message) = q.regular_message() {
                let con = cfg.redis_connection;
                let mut medicine = Medication::get_by_id(medicine_id, con.clone()).unwrap();
                medicine.set_taken_now(con.clone())?;

                let patient = Patient::get_by_id(&patient_id, con.clone()).unwrap();

                bot.edit_message_text(
                    message.chat.id,
                    message.id,
                    format!(
                        "{} has just taken {} ({}). All the best for them.",
                        patient.name, medicine.medicine, medicine.dosage
                    ),
                )
                .await?;

                // let medicines =
                //     Medication::get_all_by_patient_id(&medicine.patient_id, con.clone());

                // bot.send_message(
                //     message.chat.id,
                //     medicines
                //         .iter()
                //         .map(|m| m.print_in_list() + "\n")
                //         .collect::<String>(),
                // )
                // .await?;

                let patient = Patient::get_by_id(&patient_id, con.clone()).unwrap();
                for telegram_user in patient.get_all_shared_users() {
                    if telegram_user == q.from.id.to_string() {
                        continue;
                    }

                    match bot
                        .send_message(
                            telegram_user.clone(),
                            format!(
                                "{} just taken {} ({}). FYI!",
                                patient.name, medicine.medicine, medicine.dosage
                            ),
                        )
                        .await
                    {
                        Err(e) => {
                            log::warn!(
                                "Failed to notify shared user of intake: telegram user id {}. Error {}",
                                &telegram_user, e
                            )
                        }
                        _ => {}
                    }
                }

                dialogue.exit().await?;
            }
        }
    }

    Ok(())
}

pub async fn take_medicine_command(
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
