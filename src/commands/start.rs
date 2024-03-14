use serenity::all::{CommandInteraction, CommandOptionType, Context, CreateCommandOption};
use serenity::builder::CreateCommand;

#[allow(unused)]
pub async fn run(command: &CommandInteraction, ctx: &Context) -> String {
    // println!("Interaction received: {:?}", command.data.name);
    println!("inside Start command");

    // let data = CreateInteractionResponseMessage::new().content("Starting Connection");

    // let builder = EditInteractionResponse::new().content(" You are in queue");
    // if let Err(why) = command.edit_response(&ctx.http, builder).await {
    //     println!("Error sending response: {:?}", why);
    // }
    "starting conversation".to_string()
}

pub fn register() -> CreateCommand {
    CreateCommand::new("start")
        .description("Start convesation with random person.")
        .set_options(vec![CreateCommandOption::new(
            CommandOptionType::String,
            "interest",
            "Interest of the person",
        )
        .required(true)])
}
