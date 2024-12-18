use std::error::Error;

use crate::{
    medication::Medication, patient::Patient, Command, ConfigParameters, HandlerResult, MyDialogue,
};
use medibot::State;
use teloxide::{
    prelude::*,
    types::{InlineKeyboardMarkup, Message, ParseMode},
    utils::command::BotCommands,
    Bot,
};

pub async fn start(bot: Bot, _dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id,
        "*Welcome to 💊 MediBot\\! 💉*\nI'll remind you of when to take your meds\\.\n\n".to_owned() +
        "Add patients and medication plans with /addmedication\\. Register an intake with /take\\.\n\n" +
        "Type /help to see all available commands\\."
    )
    .parse_mode(ParseMode::MarkdownV2)

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

pub async fn get_all(
    cfg: ConfigParameters,
    bot: Bot,
    _: MyDialogue,
    msg: Message,
) -> HandlerResult {
    let con = cfg.redis_connection.clone();

    let all_patients: Vec<Patient> =
        Patient::get_my_patients(&msg.chat.id.to_string(), con.clone()).unwrap();

    let all_records: Vec<Medication> = all_patients
        .into_iter()
        .flat_map(|patient| Medication::get_all_by_patient_id(&patient.id, con.clone()))
        .collect();

    // TODO: show patient and then medicine under them
    bot.send_message(
        msg.chat.id,
        all_records
            .iter()
            .map(|m| m.print_in_list() + "\n")
            .collect::<String>(),
    )
    .await?;

    Ok(())
}

pub async fn list_my_patients(
    cfg: ConfigParameters,
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
) -> HandlerResult {
    let con = cfg.redis_connection;

    let keyboard = Patient::generate_patient_keyboard(con.clone(), msg.chat.id.to_string(), true);

    bot.send_message(
        msg.chat.id,
        "Here are the patients you have access to\\. Select one to see all meds for that patient\\.",
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

            if let Some(message) = q.regular_message() {
                let patient = Patient::get_by_id(patient_id, cfg.redis_connection.clone()).unwrap();

                let medicines =
                    Medication::get_all_by_patient_id(patient_id, cfg.redis_connection.clone());

                let msg = match medicines.len() {
                    0 => format!(
                        "No medications added yet for {}, add one with /addmedication.",
                        patient.name
                    ),
                    _ => {
                        let msg = medicines
                            .iter()
                            .map(|m| m.print_in_list() + "\n")
                            .collect::<String>();

                        format!("{}\nRegister a taken dosage by running /take.", msg)
                    }
                };
                bot.edit_message_text(message.chat.id, message.id, msg)
                    .await?;

                dialogue.exit().await?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]

mod tests {
    use super::*;
    use crate::frequency::Frequency;
    use std::sync::{Arc, Mutex};

    fn create_test_redis_connection() -> redis::Connection {
        // creating a real connection actually
        let client = redis::Client::open("redis://127.0.0.1/").expect("Could not connect to Redis");
        let mut redis_connection = client
            .get_connection()
            .expect("Could not get a Redis connection");

        redis::cmd("SELECT")
            .arg(1) // selecting db 1 for tests to preserve data on the other one (default, 0)
            .exec(&mut redis_connection)
            .unwrap();

        redis::cmd("FLUSHDB").exec(&mut redis_connection).unwrap();

        redis_connection
    }

    #[test]
    fn test_get_all() {
        let user_id = uuid::Uuid::new_v4().to_string();

        let patient = Patient::new("xavi".to_string(), user_id.clone());

        let mut medication = Medication::new(
            patient.id.clone(),
            "nurofen".to_string(),
            "5ml".to_string(),
            Frequency::new(3),
            user_id.clone(),
        );

        let redis_con = Arc::new(Mutex::new(create_test_redis_connection()));

        patient.save(redis_con.clone()).unwrap();
        let res = medication.save(redis_con.clone());

        assert!(res.is_ok());

        let all_records: Vec<Medication> =
            Medication::get_all_by_patient_id(&patient.id, redis_con.clone());
        assert_eq!(all_records.len(), 1);

        let other_records: Vec<Medication> =
            Medication::get_all_by_patient_id(&"hello", redis_con.clone());
        assert_eq!(other_records.len(), 0);
    }
}
