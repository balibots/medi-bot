use crate::{
    commands::{cancel, get_all_command, help, start},
    flows::add_medication::*,
    flows::patients::*,
    flows::take_medicine::*,
};
use dotenv::dotenv;
use dptree::filter;
use medibot::{Command, State};
use redis::Connection;
use std::{
    env,
    sync::{Arc, Mutex},
};
use teloxide::{
    dispatching::{
        dialogue::{self, InMemStorage},
        UpdateHandler,
    },
    prelude::*,
    update_listeners::webhooks,
    utils::command::BotCommands,
};

use url::Url;

mod commands;
mod err_handling;
mod flows;
mod frequency;
mod medication;
mod patient;

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone)]
pub struct ConfigParameters {
    redis_connection: Arc<Mutex<Connection>>,
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    pretty_env_logger::init();
    log::info!("Starting dialogue bot...");

    let bot = Bot::from_env();

    let client = redis::Client::open(env::var("REDIS_URL").expect("REDIS_URL missing"))
        .expect("Could not connect to Redis");

    // not sure this is working:
    bot.set_chat_menu_button()
        .menu_button(teloxide::types::MenuButton::Commands)
        .await
        .expect("Error setting chat menu button");

    bot.set_my_commands(Command::bot_commands())
        .await
        .expect("Error setting my commands");

    let redis_connection = client
        .get_connection()
        .expect("Could not get a Redis connection");

    let parameters: ConfigParameters = ConfigParameters {
        redis_connection: Arc::new(Mutex::new(redis_connection)),
    };

    let mut dispatch_builder = Dispatcher::builder(bot.clone(), schema())
        .dependencies(dptree::deps![parameters, InMemStorage::<State>::new()])
        .enable_ctrlc_handler()
        .build();

    let webhook_url = env::var("WEBHOOK_URL");

    match webhook_url {
        Ok(host) if host.len() > 0 => {
            // using webhooks
            let port: u16 = env::var("PORT")
                .expect("PORT env variable is not set")
                .parse()
                .expect("PORT env variable value is not an integer");

            log::info!("Using WebHooks, host: {}, port: {}", host, port);

            let addr = ([0, 0, 0, 0], port).into();

            // Heroku host example: "heroku-ping-pong-bot.herokuapp.com"
            let url = Url::parse(&host)
                .expect("HOST env var Url malformed")
                .join("/webhookBot") // TODO should this be token?
                .expect("Invalid WEBHOOK_URL");

            let listener = webhooks::axum(bot.clone(), webhooks::Options::new(addr, url))
                .await
                .expect("Couldn't setup webhook");

            dispatch_builder
                .dispatch_with_listener(listener, err_handling::MyErrorHandler::new())
                .await;
        }
        _ => {
            log::info!("Using long polling");

            // long polling
            dispatch_builder.dispatch().await;
        }
    }
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>().branch(
        filter(|update: Update| update.chat().unwrap().is_private())
            .branch(case![Command::Help].endpoint(help))
            .branch(case![Command::Start].endpoint(start))
            .branch(case![Command::AddMedication].endpoint(start_add_medication))
            .branch(case![Command::GetAll].endpoint(get_all_command))
            .branch(case![Command::Take].endpoint(take_medicine_command))
            .branch(case![Command::Patients].endpoint(patients_command))
            .branch(case![Command::Cancel].endpoint(cancel)),
    );

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(dptree::case![State::ReceiveName].endpoint(receive_name))
        .branch(dptree::case![State::ReceiveMedicine { patient_id }].endpoint(receive_medicine))
        .branch(
            dptree::case![State::ReceiveDosage {
                patient_id,
                medicine
            }]
            .endpoint(receive_dosage),
        )
        .branch(
            dptree::case![State::ReceiveFrequency {
                patient_id,
                medicine,
                dosage,
            }]
            .endpoint(receive_frequency),
        )
        .branch(dptree::case![State::ReceivePatientName].endpoint(receive_new_patient_name))
        .branch(
            dptree::case![State::ReceiveTelegramUserForSharePatient { patient_id }]
                .endpoint(receive_telegram_user_name),
        )
        .branch(filter(|update: Update| update.chat().unwrap().is_group()).endpoint(group_handler))
        .branch(dptree::endpoint(default_handler));

    let callback_handler = Update::filter_callback_query()
        .branch(dptree::case![State::ReceiveName].endpoint(receive_name_callback_handler))
        .branch(dptree::case![State::TakeMedicine].endpoint(take_medicine_callback_handler))
        .branch(
            dptree::case![State::TakeMedicineFinal { patient_id }]
                .endpoint(take_medicine_second_callback_handler),
        )
        .branch(dptree::case![State::SelectPatient].endpoint(select_patient_callback_handler))
        .branch(
            dptree::case![State::PatientOps { patient_id }].endpoint(patient_ops_callback_handler),
        )
        .branch(
            dptree::case![State::MedicineLog { patient_id }]
                .endpoint(medicine_log_callback_handler),
        );

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(callback_handler)
        .branch(message_handler)
}

async fn group_handler(bot: Bot, _dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        "Sorry I can only reply to private messages, come and have a chat! :)",
    )
    .await?;
    Ok(())
}

async fn default_handler(bot: Bot, _dialogue: MyDialogue, msg: Message) -> HandlerResult {
    log::info!("{:?}\n\n{:?}", _dialogue.get().await?.unwrap(), msg);
    bot.send_message(
        msg.chat.id,
        "Didn't quite get that. Try /addmedication, /patients or /help!",
    )
    .await?;
    Ok(())
}
