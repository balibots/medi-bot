use teloxide::macros::BotCommands;

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    ReceiveName,
    ReceiveMedicine {
        patient_id: String,
    },
    ReceiveDosage {
        patient_id: String,
        medicine: String,
    },
    ReceiveFrequency {
        patient_id: String,
        medicine: String,
        dosage: String,
    },

    StartAddPatient,
    ReceivePatientName,
    TakeMedicine,
    TakeMedicineFinal {
        patient_id: String,
    },
    SelectPatient,
    StartSharePatient,
    ReceiveTelegramUserForSharePatient {
        patient_id: String,
    },
    PatientOps {
        patient_id: String,
    },
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:",
    command_separator = "_"
)]
pub enum Command {
    #[command(description = "start interacting with the bot.")]
    Start,
    #[command(description = "display this text.")]
    Help,
    #[command(description = "manage my patients")]
    Patients,
    #[command(description = "add a new medication plan.")]
    AddMedication,
    #[command(description = "register a medicine being taken")]
    Take,
    #[command(description = "cancel the current operation.")]
    Cancel,
    #[command(description = "gets all the patients and meds you have access to.")]
    GetAll,
}
