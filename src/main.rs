use crate::{
    add_medication::*,
    add_patient::*,
    commands::{cancel, get_all, help, start},
    take_medicine::*,
};
use commands::{list_my_patients, select_patient_callback_handler};
use dotenv::dotenv;
use medibot::{Command, State};
use redis::Connection;
use std::sync::{Arc, Mutex};
use teloxide::{
    dispatching::{dialogue, dialogue::InMemStorage, UpdateHandler},
    prelude::*,
};

mod add_medication;
mod add_patient;
mod commands;
mod frequency;
mod medication;
mod patient;
mod take_medicine;

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

    let client = redis::Client::open("redis://127.0.0.1/").expect("Could not connect to Redis");
    let redis_connection = client
        .get_connection()
        .expect("Could not get a Redis connection");

    let parameters: ConfigParameters = ConfigParameters {
        redis_connection: Arc::new(Mutex::new(redis_connection)),
    };

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![parameters, InMemStorage::<State>::new()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(
            case![State::Start]
                .branch(case![Command::Help].endpoint(help))
                .branch(case![Command::Start].endpoint(start))
                .branch(case![Command::AddMedication].endpoint(start_add_medication))
                .branch(case![Command::GetAll].endpoint(get_all))
                .branch(case![Command::Take].endpoint(take_medicine))
                .branch(case![Command::Patients].endpoint(list_my_patients))
                .branch(case![Command::AddPatient].endpoint(start_add_patient)),
        )
        .branch(case![Command::Cancel].endpoint(cancel));

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
        .branch(dptree::case![State::ReceivePatientName].endpoint(receive_patient_name))
        .branch(dptree::endpoint(default_handler));

    let callback_handler = Update::filter_callback_query()
        .branch(dptree::case![State::ReceiveName].endpoint(receive_name_callback_handler))
        .branch(dptree::case![State::TakeMedicine].endpoint(take_medicine_callback_handler))
        .branch(
            dptree::case![State::TakeMedicineFinal { patient_id }]
                .endpoint(take_medicine_second_callback_handler),
        )
        .branch(dptree::case![State::SelectPatient].endpoint(select_patient_callback_handler));

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(callback_handler)
        .branch(message_handler)
}

async fn default_handler(bot: Bot, _dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        "Didn't quite get that. Try /addmedication or /help!",
    )
    .await?;
    Ok(())
}
