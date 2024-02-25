// use std::env;

mod commands;

use serenity::all::{
    Command, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseFollowup,
    CreateInteractionResponseMessage, CreateMessage, Interaction, Timestamp,
};
use serenity::async_trait;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

#[derive(Debug)]
struct UserData {
    connecting: Vec<String>,
    connected: Vec<String>,
    pairs: Vec<(String, String)>,
}

static USER_DATA: std::sync::Mutex<UserData> = std::sync::Mutex::new(UserData {
    connecting: Vec::new(),
    connected: Vec::new(),
    pairs: Vec::new(),
});
struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            println!("Interaction received: {:?}", command);
            println!("from : {:?}", command.user.global_name.clone().unwrap());

            // if command
            //     .user
            //     .has_role(&ctx.http, 696775407764242523, 69677885235304827)
            //     .await
            //     .unwrap()
            // {
            //     println!("User has role ");
            // } else {
            //     println!("User does not have role");
            // }

            // let guild_id = GuildId::new(696775407764242523);
            // let role_id = RoleId::new(696778852353048627);

            // println!(
            //     "roles: {:?}",
            //     guild_id
            //         .roles(&ctx.http)
            //         .await
            //         .unwrap()
            //         .get(&role_id)
            //         .unwrap()
            //         .name
            // );

            let content = match command.data.name.as_str() {
                "pinga" => Some(commands::ping::run(&command.data.options())),
                "start" => {
                    println!(
                        "Interaction received: {:?}",
                        command.user.global_name.clone().unwrap()
                    );

                    let mut user_data = USER_DATA.lock().unwrap();

                    if user_data
                        .connecting
                        .contains(&command.user.global_name.clone().unwrap())
                    {
                        println!("You are already in queue");
                        Some("You are already in queue".to_string())
                    } else if user_data
                        .connected
                        .contains(&command.user.global_name.clone().unwrap())
                    {
                        Some("You are already connected".to_string())
                    } else {
                        if user_data.connecting.len() > 0 {
                            let user = user_data.connecting.pop().unwrap();
                            user_data.connected.push(user.clone());
                            user_data
                                .connected
                                .push(command.user.global_name.clone().unwrap());
                            user_data
                                .pairs
                                .push((user.clone(), command.user.global_name.clone().unwrap()));
                            user_data
                                .pairs
                                .push((command.user.global_name.clone().unwrap(), user.clone()));
                        } else {
                            user_data
                                .connecting
                                .push(command.user.global_name.clone().unwrap());
                        }
                        println!("connected: {:?}", user_data.connected);
                        println!("connecting: {:?}", user_data.connecting);
                        println!("pairs: {:?}", user_data.pairs);
                        Some(commands::start::run(
                            &command.data.options(),
                            &command,
                            &ctx,
                        ))
                    }
                }
                _ => Some("Unknown command".to_string()),
            };

            if let Some(content) = content {
                let data = CreateInteractionResponseMessage::new()
                    .content(content)
                    .embed(
                        CreateEmbed::new()
                            .title("Title")
                            .description("Description lorem ipsum")
                            .fields(vec![
                                ("This is the first field", "This is a field body", true),
                                ("This is the second field", "Both fields are inline", true),
                            ])
                            .timestamp(Timestamp::now()),
                    );
                let builder = CreateInteractionResponse::Message(data);

                if let Err(why) = command.create_response(&ctx.http, builder).await {
                    println!("Cannot respond to slash command: {why}");
                }
                if let Err(why) = command
                    .create_followup(
                        &ctx.http,
                        CreateInteractionResponseFollowup::new()
                            .content("Test Follow up".to_string())
                            .ephemeral(true),
                    )
                    .await
                {
                    println!("Cannot respond to slash command: {why}");
                }
                let builder = CreateMessage::new().content("Hello from dms");
                // command.user.direct_message(&ctx.http, builder);
                if let Err(why) = command.user.direct_message(&ctx, builder).await {
                    println!("Err sending help: {why:?}");
                };
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        let guild_command =
            Command::create_global_command(&ctx.http, commands::ping::register()).await;

        let guild_command2 =
            Command::create_global_command(&ctx.http, commands::start::register()).await;

        println!("I created the following global slash command: {guild_command:#?}");
        println!("I created the following global slash command: {guild_command2:#?}");
    }
}

#[tokio::main]
async fn main() {
    // Configure the client with your Discord bot token in the environment.
    let token =
        std::env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as a bot. This will automatically prepend
    // your bot token with "Bot ", which is a requirement by Discord for bot users.
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform exponential backoff until
    // it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
