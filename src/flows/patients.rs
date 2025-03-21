use crate::commands::cancel_with_edit;
use crate::medication::Medication;
use crate::user::get_user_timezone;
use crate::{patient::Patient, ConfigParameters, HandlerResult, MyDialogue, State};
use chrono::DateTime;
use chrono_tz::Tz;
use std::error::Error;
use teloxide::payloads::EditMessageTextSetters;
use teloxide::prelude::*;
use teloxide::types::{
    ButtonRequest, CallbackQuery, KeyboardButton, KeyboardButtonRequestUsers, KeyboardRemove,
};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};
use teloxide::types::{KeyboardMarkup, Message};
use teloxide::Bot;

pub async fn patients_command(
    cfg: ConfigParameters,
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
) -> HandlerResult {
    let con = cfg.redis_connection;

    let keyboard = Patient::generate_patient_keyboard(con.clone(), msg.chat.id.to_string(), true);

    bot.send_message(
        msg.chat.id,
        "🤒 Here are the patients you have access to\\. 🤕\n\nSelect one for more options or add a new one below:"
    )
    .reply_markup(InlineKeyboardMarkup::new(keyboard))
    .parse_mode(ParseMode::MarkdownV2)
    .await?;

    dialogue.update(State::SelectPatient).await?;
    Ok(())
}

pub async fn select_patient_callback_handler(
    cfg: ConfigParameters,
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let message = q.regular_message().unwrap();
    if let Some(ref patient_id) = q.data {
        bot.answer_callback_query(&q.id).await?;

        if patient_id == "cancel" {
            cancel_with_edit(bot, dialogue, message.to_owned()).await?;
        } else if patient_id == "add_new" {
            bot.edit_message_text(
                message.chat.id,
                message.id,
                "Ok, adding a new patient. Please tell me what's their name:",
            )
            .await?;
            dialogue.update(State::ReceivePatientName).await?;
        } else {
            let patient = Patient::get_by_id(patient_id, cfg.redis_connection.clone()).unwrap();
            let keyboard: Vec<Vec<InlineKeyboardButton>> = vec![
                vec![InlineKeyboardButton::callback(
                    "Register medicine intake ".to_string(),
                    "take".to_string(),
                )],
                vec![
                    InlineKeyboardButton::callback(
                        "All medications".to_string(),
                        "list_medication".to_string(),
                    ),
                    InlineKeyboardButton::callback(
                        "Intake log".to_string(),
                        "medication_log".to_string(),
                    ),
                ],
                vec![
                    InlineKeyboardButton::callback(
                        "Share".to_string(),
                        "share_patient".to_string(),
                    ),
                    InlineKeyboardButton::callback(
                        "Delete".to_string(),
                        "delete_patient".to_string(),
                    ),
                ],
                vec![InlineKeyboardButton::callback(
                    "Cancel".to_string(),
                    "cancel".to_string(),
                )],
            ];

            let sharing = patient.get_shared_with();
            let shared_msg = if sharing.len() > 0 {
                format!(
                    "Patient shared with accounts: {}\\.\n\n",
                    sharing
                        .iter()
                        .fold(String::new(), |acc, id| { acc + &id.to_string() })
                )
            } else {
                "".to_string()
            };

            bot.edit_message_text(
                message.chat.id,
                message.id,
                format!("{}Select optzionne for {}:", shared_msg, patient.name),
            )
            .reply_markup(InlineKeyboardMarkup::new(keyboard))
            .parse_mode(ParseMode::MarkdownV2)
            .await?;

            dialogue
                .update(State::PatientOps {
                    patient_id: patient_id.to_string(),
                })
                .await?;
        }
    }

    Ok(())
}

pub async fn patient_ops_callback_handler(
    cfg: ConfigParameters,
    bot: Bot,
    dialogue: MyDialogue,
    patient_id: String,
    q: CallbackQuery,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let message = q.regular_message().unwrap();

    if let Some(ref op) = q.data {
        bot.answer_callback_query(&q.id).await?;
        let con = cfg.redis_connection;
        let patient = Patient::get_by_id(&patient_id, con.clone()).unwrap();

        if op == "cancel" {
            cancel_with_edit(bot, dialogue, message.to_owned()).await?;
        } else if op == "take" {
            let new_keyb = Medication::generate_medication_keyboard(&patient_id, con.clone());

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
        } else if op == "share_patient" {
            let btn = KeyboardButton::new("Select user");
            let btnrequest = btn.request(ButtonRequest::RequestUsers(
                KeyboardButtonRequestUsers::new(teloxide::types::RequestId(1)),
            ));
            let keyb = KeyboardMarkup::new(vec![vec![btnrequest]]);

            bot.send_message(
                message.chat.id,
                format!(
                    "Great. Now please select the Telegram user you'll be sharing {} with or /cancel.",
                    patient.name
                ),
            )
            .reply_markup(keyb.one_time_keyboard())
            .await?;
            dialogue
                .update(State::ReceiveTelegramUserForSharePatient {
                    patient_id: patient_id.into(),
                })
                .await?;
        } else if op == "delete_patient" {
            let patient = Patient::get_by_id(&patient_id, con.clone()).expect("Patient not found");
            patient.delete(con.clone())?;
            bot.edit_message_text(
                message.chat.id,
                message.id,
                format!("Patient {} deleted.", patient.name),
            )
            .await?;
            dialogue.exit().await?;
        } else if op == "list_medication" {
            let patient = Patient::get_by_id(&patient_id, con.clone()).expect("Patient not found");
            let mut medicines = Medication::get_all_by_patient_id(&patient_id, con.clone());

            medicines.sort_by_key(|m| m.last_taken);
            medicines.reverse();

            let msg = match medicines.len() {
                0 => format!(
                    "No medications added yet for {}, add one with /addmedication.",
                    patient.name
                ),
                _ => {
                    let tz = get_user_timezone(con.clone(), &message.chat.id.to_string());

                    let msg = medicines
                        .iter()
                        .map(|m| m.print_in_list(&tz) + "\n")
                        .collect::<String>();

                    format!("{}\nRegister a taken dosage by running /take.", msg)
                }
            };
            bot.edit_message_text(message.chat.id, message.id, msg)
                .await?;

            dialogue.exit().await?;
        } else if op == "medication_log" {
            let new_keyb = Medication::generate_medication_keyboard(&patient_id, con.clone());

            bot.edit_message_text(
                message.chat.id,
                message.id,
                "Great, getting the log for which medicine?",
            )
            .reply_markup(InlineKeyboardMarkup::new(new_keyb))
            .parse_mode(ParseMode::MarkdownV2)
            .await?;

            dialogue
                .update(State::MedicineLog {
                    patient_id: patient_id.into(),
                })
                .await?;
        } else {
            bot.edit_message_text(message.chat.id, message.id, "Didn't quite get that, sorry.")
                .await?;
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
    match msg.shared_users() {
        Some(users) => {
            log::info!("shared users {:?}", users);
            let con = cfg.redis_connection;

            let mut patient = Patient::get_by_id(&patient_id, con.clone())
                .expect("Error getting patient for sharing");

            users.user_ids.iter().for_each(|id| {
                Patient::share(&mut patient, id.0, con.clone()).unwrap();
            });

            patient
                .save(con.clone())
                .expect("Error saving patient after sharing");

            bot.send_message(msg.chat.id, format!("Patient {} shared.", patient.name))
                .await?;

            dialogue.exit().await?;
        }
        None => match msg.text() {
            Some(text) if text == "/cancel" => {
                bot.send_message(msg.chat.id, "Cancelling the current operation.")
                    .reply_markup(KeyboardRemove::new())
                    .await?;
                dialogue.exit().await?;
            }
            Some(text) if text.parse::<u64>().is_ok() => {
                let con = cfg.redis_connection;
                let parsed_id = text.parse::<u64>().unwrap();

                let mut patient = Patient::get_by_id(&patient_id, con.clone())
                    .expect("Error getting patient for sharing");

                Patient::share(&mut patient, parsed_id, con.clone())?;

                patient
                    .save(con.clone())
                    .expect("Error saving patient after sharing");

                bot.send_message(msg.chat.id, "Patient shared.")
                    .reply_markup(KeyboardRemove::new())
                    .await?;

                dialogue.exit().await?;
            }
            _ => {
                bot.send_message(msg.chat.id, "That doesn't look like a Telegram User ID (should be a number). Please try again or /cancel.")
                        .await?;
            }
        },
    }

    Ok(())
}

pub async fn receive_new_patient_name(
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
                    "Added patient {}. You might want to /addmedication next.",
                    text
                ),
            )
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

pub async fn medicine_log_callback_handler(
    cfg: ConfigParameters,
    bot: Bot,
    dialogue: MyDialogue,
    patient_id: String,
    q: CallbackQuery,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let message = q.regular_message().unwrap();
    bot.answer_callback_query(&q.id).await?;

    if let Some(ref medication_id) = q.data {
        if medication_id == "cancel" {
            cancel_with_edit(bot, dialogue, message.to_owned()).await?;
        } else {
            let con = cfg.redis_connection;
            let patient = Patient::get_by_id(&patient_id, con.clone()).unwrap();
            let medication = Medication::get_by_id(medication_id, con.clone()).unwrap();
            let log = medication.get_medication_log(con.clone()).unwrap();

            let header = format!(
                "Log for {} administration of {} ({}):\n",
                patient.name, medication.medicine, medication.dosage
            );

            if log.len() == 0 {
                bot.edit_message_text(
                    message.chat.id,
                    message.id,
                    format!(
                        "{}\n{}",
                        header, " - Patient hasn't taken this medicine yet."
                    ),
                )
                .await?;
            } else {
                let tz = get_user_timezone(con.clone(), &message.chat.id.to_string());
                let timezone: Tz = tz.parse().unwrap();
                bot.edit_message_text(
                    message.chat.id,
                    message.id,
                    format!(
                        "{}\n{}",
                        header,
                        log.into_iter()
                            .map(|ts| format!(
                                " - {}\n",
                                DateTime::from_timestamp(ts, 0)
                                    .unwrap()
                                    .with_timezone(&timezone)
                                    .to_string()
                            ))
                            .collect::<String>()
                    ),
                )
                .await?;
            }
            dialogue.exit().await?;
        }
    }

    Ok(())
}
