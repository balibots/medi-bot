use crate::{
    medication::Medication, patient::Patient, user::get_user_timezone, Command, ConfigParameters,
    HandlerResult, MyDialogue,
};
use chrono_tz::Tz;
use redis::Commands;
use teloxide::{
    prelude::*,
    types::{KeyboardRemove, Message, ParseMode},
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
        .reply_markup(KeyboardRemove::new())
        .await?;
    dialogue.exit().await?;
    Ok(())
}

pub async fn cancel_with_edit(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.edit_message_text(msg.chat.id, msg.id, "Cancelling the current operation.")
        .await?;
    dialogue.exit().await?;
    Ok(())
}

pub async fn get_all_command(
    cfg: ConfigParameters,
    bot: Bot,
    _: MyDialogue,
    msg: Message,
) -> HandlerResult {
    let con = cfg.redis_connection.clone();

    let all_patients: Vec<Patient> =
        Patient::get_my_patients(&msg.chat.id.to_string(), con.clone()).unwrap();

    let outgoing_msg = all_patients
        .iter()
        .map(|p| {
            let meds = Medication::get_all_by_patient_id(&p.id, con.clone());

            let tz = get_user_timezone(con.clone(), &msg.chat.id.to_string());

            let listprint = match meds.len() {
                0 => " - No medications taken yet.\n".to_string(),
                _ => meds
                    .iter()
                    .map(|m| m.print_in_list(&tz) + "\n")
                    .collect::<String>(),
            };

            format!("*{}*\n{}\n", p.name, listprint)
        })
        .collect::<String>();

    if outgoing_msg.len() == 0 {
        bot.send_message(
            msg.chat.id,
            "No medications added yet - try /addmedication to start.",
        )
        .await?;
    } else {
        bot.send_message(msg.chat.id, outgoing_msg).await?;
    }

    Ok(())
}

pub async fn set_timezone(
    cfg: ConfigParameters,
    bot: Bot,
    _: MyDialogue,
    timezone: String,
    msg: Message,
) -> HandlerResult {
    let tz: Result<Tz, chrono_tz::ParseError> = timezone.parse();
    match tz {
        Ok(_) => {
            // save to redis
            let con = cfg.redis_connection.clone();

            con.lock().unwrap().set::<String, String, ()>(
                format!("medi:{}:timezone", msg.from.unwrap().id),
                timezone.clone(),
            )?;

            bot.send_message(msg.chat.id, format!("Timezone set for {}", &timezone))
                .await?;
        }
        Err(_) => {
            bot.send_message(
                msg.chat.id,
                format!("Sorry, I don't recognise the timezone: {}", timezone),
            )
            .await?;
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
