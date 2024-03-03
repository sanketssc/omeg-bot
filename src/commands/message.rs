use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommandOption,
    CreateInteractionResponseMessage,
};
use serenity::builder::CreateCommand;

#[allow(unused)]
pub async fn run(command: &CommandInteraction, ctx: &Context) -> String {
    // println!("Interaction received: {:?}", command.data.name);

    let data = CreateInteractionResponseMessage::new().content("Starting Connection");

    // let builder = EditInteractionResponse::new().content(" You are in queue");
    // if let Err(why) = command.edit_response(&ctx.http, builder).await {
    //     println!("Error sending response: {:?}", why);
    // }
    "starting conversation".to_string()
}

pub fn register() -> CreateCommand {
    CreateCommand::new("message")
        .description("send message to connected person.")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "message", "message to send")
                .required(true),
        )
}
