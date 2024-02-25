use serenity::all::{CommandInteraction, Context};
use serenity::builder::CreateCommand;
use serenity::model::application::ResolvedOption;

#[allow(unused_variables)]
pub fn run(_options: &[ResolvedOption], command: &CommandInteraction, ctx: &Context) -> String {
    // println!("Interaction received: {:?}", command.data.name);

    "starting conversation".to_string()
}

pub fn register() -> CreateCommand {
    CreateCommand::new("start").description("Start convesation with random person.")
}
