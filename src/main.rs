use std::sync::{Arc, Mutex};

use crate::add_medication::*;
use crate::commands::{cancel, help, start};
use dotenv::dotenv;
use medibot::{Command, State};
use redis::Connection;
use teloxide::{
    dispatching::{
        dialogue::{self, InMemStorage},
        UpdateHandler,
    },
    prelude::*,
};

mod add_medication;
mod commands;
mod frequency;
mod medication;

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
                .branch(case![Command::AddMedication].endpoint(/*start_add_medication*/ test_add)),
        )
        .branch(case![Command::Cancel].endpoint(cancel));

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(dptree::case![State::ReceiveName].endpoint(receive_name))
        .branch(dptree::case![State::ReceiveMedicine { name }].endpoint(receive_medicine))
        .branch(dptree::case![State::ReceiveDosage { name, medicine }].endpoint(receive_dosage))
        .branch(
            dptree::case![State::ReceiveFrequency {
                name,
                medicine,
                dosage,
            }]
            .endpoint(receive_frequency),
        )
        .branch(dptree::endpoint(default_handler));

    dialogue::enter::<Update, InMemStorage<State>, State, _>().branch(message_handler)
}

async fn default_handler(bot: Bot, _dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        "Didn't quite get that. Try /addmedication or /help!",
    )
    .await?;
    Ok(())
}
