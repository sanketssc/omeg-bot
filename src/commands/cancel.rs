use serenity::all::CreateCommand;

pub fn register() -> CreateCommand {
    CreateCommand::new("cancel").description("Cancel current conversation.")
}
