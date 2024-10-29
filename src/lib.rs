use teloxide::utils::command::BotCommands;

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    ReceiveName,
    ReceiveMedicine {
        name: String,
    },
    ReceiveDosage {
        name: String,
        medicine: String,
    },
    ReceiveFrequency {
        name: String,
        medicine: String,
        dosage: String,
    },
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    #[command(description = "start interacting with the bot.")]
    Start,
    #[command(description = "display this text.")]
    Help,
    #[command(description = "add a new medication regimen.")]
    AddMedication,
    #[command(description = "cancel the current operation.")]
    Cancel,
}
