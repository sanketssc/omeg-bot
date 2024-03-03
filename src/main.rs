// use std::env;

mod commands;

use serenity::all::{
    ActivityData, CommandInteraction, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage, EditInteractionResponse, Guild, Interaction, UserId,
};
use serenity::async_trait;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

use redis::{Commands, PubSubCommands};

struct Handler;

fn matcher(command: &CommandInteraction, redis_connection: &mut redis::Connection) -> String {
    let connecting: String = redis_connection.get("connecting").unwrap();
    let connected: String = redis_connection.get("connected").unwrap();

    let mut connecting_vec: Vec<UserId> = serde_json::from_str(&connecting).unwrap();
    let mut connected_vec: Vec<UserId> = serde_json::from_str(&connected).unwrap();

    if let Some(_val) = connecting_vec.iter().find(|_id| _id == &&command.user.id) {
        println!("You are already in queue");

        return "You are already in queue".to_string();
    }

    if let Some(_val) = connected_vec.iter().find(|_id| _id == &&command.user.id) {
        println!("You are already connected");
        return "You are already connected".to_string();
    }

    if connecting_vec.len() > 0 {
        let free_user = connecting_vec[0];
        connecting_vec.remove(0);
        connected_vec.push(free_user);
        connected_vec.push(command.user.id);
        let connected_ser = serde_json::to_string(&connected_vec).unwrap();
        let connecting_ser = serde_json::to_string(&connecting_vec).unwrap();
        let _: Result<String, redis::RedisError> = redis_connection.set("connected", connected_ser);
        let _: Result<String, redis::RedisError> =
            redis_connection.set("connecting", connecting_ser);
        let _: Result<String, redis::RedisError> =
            redis_connection.set(command.user.id.to_string(), free_user.to_string());
        let _: Result<String, redis::RedisError> =
            redis_connection.set(free_user.to_string(), command.user.id.to_string());

        return "You are connected".to_string();
    }

    connecting_vec.push(command.user.id);

    println!("Subscribed to user: {:?}", command.user.id);
    let connecting_ser = serde_json::to_string(&connecting_vec).unwrap();
    let _: Result<String, redis::RedisError> = redis_connection.set("connecting", connecting_ser);
    "You are in queue".to_string()
}

fn get_connection_user(
    id: UserId,
    redis_connection: &mut redis::Connection,
) -> Result<UserId, redis::RedisError> {
    let conn_user: u64 = redis_connection.get(id.to_string())?;
    Ok(UserId::new(conn_user))
}

fn disconnect_users(user1: UserId, user2: UserId, mut redis_connection: redis::Connection) {
    let _: Result<String, redis::RedisError> = redis_connection.del(user1.to_string());
    let _: Result<String, redis::RedisError> = redis_connection.del(user2.to_string());

    let connected: String = redis_connection.get("connected").unwrap();
    let mut connected_vec: Vec<UserId> = serde_json::from_str(&connected).unwrap();
    connected_vec.retain(|user| user != &user1 && user != &user2);
    let connected_ser = serde_json::to_string(&connected_vec).unwrap();
    let _: Result<String, redis::RedisError> = redis_connection.set("connected", connected_ser);

    let connecting: String = redis_connection.get("connecting").unwrap();
    let mut connecting_vec: Vec<UserId> = serde_json::from_str(&connecting).unwrap();
    connecting_vec.retain(|user| user != &user1 && user != &user2);
    let connecting_ser = serde_json::to_string(&connecting_vec).unwrap();
    let _: Result<String, redis::RedisError> = redis_connection.set("connecting", connecting_ser);
}

fn get_redis_connection() -> Result<redis::Connection, redis::RedisError> {
    let client = redis::Client::open(
        std::env::var("REDIS_URL").expect("REDIS_URL must be set in the environment"),
    )?;
    let mut con: redis::Connection = client.get_connection()?;
    let _ = con.set("connected", "true")?;
    Ok(con)
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            let redis_connection = get_redis_connection();
            match &redis_connection {
                Ok(_con) => {
                    println!("Connected to redis");
                }
                Err(e) => {
                    println!("Error connecting to redis : {:?}", e);
                    command
                        .create_response(
                            ctx.http,
                            CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new()
                                    .content("There was some error connecting to redis"),
                            ),
                        )
                        .await
                        .unwrap();
                    return;
                }
            }

            let mut redis_connection = redis_connection.unwrap();

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

            match command.data.name.as_str() {
                "pinga" => Some(commands::ping::run(&command.data.options())),
                "start" => {
                    println!(
                        "Interaction received: {:?}",
                        command.user.global_name.clone().unwrap()
                    );
                    command.defer_ephemeral(&ctx.http).await.unwrap();
                    command
                        .edit_response(
                            &ctx.http,
                            EditInteractionResponse::new().content("Starting Connection"),
                        )
                        .await
                        .unwrap();

                    let response = matcher(&command, &mut redis_connection);
                    command
                        .edit_response(&ctx.http, EditInteractionResponse::new().content(response))
                        .await
                        .unwrap();

                    // commands::start::run(&command, &ctx).await;
                    Some("Ok".to_string())
                }
                "message" => {
                    let message = command
                        .data
                        .options
                        .get(0)
                        .unwrap()
                        .value
                        .as_str()
                        .unwrap()
                        .to_string();

                    if let Err(_) = get_connection_user(command.user.id, &mut redis_connection) {
                        command
                            .create_response(
                                &ctx.http,
                                CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::new().
                                    content("You are currently not connected use /start to connect to user")),
                            )
                            .await
                            .unwrap();
                        return;
                    }
                    let user = get_connection_user(command.user.id, &mut redis_connection).unwrap();
                    println!("154, {:?}", user);

                    user.create_dm_channel(&ctx.http)
                        .await
                        .unwrap()
                        .send_message(&ctx.http, CreateMessage::new().content(message))
                        .await
                        .unwrap();

                    command
                        .create_response(
                            &ctx.http,
                            CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new()
                                    .content("Message sent")
                                    .ephemeral(true),
                            ),
                        )
                        .await
                        .unwrap();

                    Some("Ok".to_string())
                }
                "leave" => {
                    let user = command.user.id;

                    if let Err(_) = get_connection_user(user, &mut redis_connection) {
                        command
                            .create_response(
                                &ctx.http,
                                CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::new()
                                        .content("You are currently not connected"),
                                ),
                            )
                            .await
                            .unwrap();
                        return;
                    }
                    let user2 = get_connection_user(user, &mut redis_connection).unwrap();
                    disconnect_users(user, user2, redis_connection);

                    command
                        .create_response(
                            &ctx.http,
                            CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new()
                                    .content("You left the conversation")
                                    .ephemeral(true),
                            ),
                        )
                        .await
                        .unwrap();

                    user.create_dm_channel(&ctx.http)
                        .await
                        .unwrap()
                        .send_message(
                            &ctx.http,
                            CreateMessage::new()
                                .content("You have left the conversation with sranger"),
                        )
                        .await
                        .unwrap();

                    user2
                        .create_dm_channel(&ctx.http)
                        .await
                        .unwrap()
                        .send_message(
                            &ctx.http,
                            CreateMessage::new().content("Other user left the conversation"),
                        )
                        .await
                        .unwrap();

                    Some("Ok".to_string())
                }
                _ => Some("Unknown command".to_string()),
            };
        }
    }
    async fn guild_create(&self, _ctx: Context, guild: Guild, is_new: Option<bool>) {
        if is_new.unwrap() {
            println!("Bot joined a new guild:");
            println!("Guild Name: {}", guild.name);
            println!("Guild ID: {}", guild.id);
            println!("---------------------");
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        let cache = ctx.cache.guilds();
        println!("{cache:#?}");

        ctx.set_activity(Some(ActivityData::playing("Bot-Bot")));
        // for command in Command::get_global_commands(&ctx.http)
        //     .await
        //     .unwrap()
        //     .iter()
        // {
        //     println!("Command: {:?}", command.id);
        //     Command::delete_global_command(&ctx.http, command.id)
        //         .await
        //         .unwrap();
        // }

        // let _ = Command::create_global_command(&ctx.http, commands::ping::register()).await;

        // let _ = Command::create_global_command(&ctx.http, commands::start::register()).await;

        // let _ = Command::create_global_command(&ctx.http, commands::message::register()).await;

        // let _ = Command::create_global_command(&ctx.http, commands::leave::register()).await;

        // println!("I created the following global slash command: {guild_command:#?}");
        // println!("I created the following global slash command: {guild_command2:#?}");
    }
}

#[tokio::main]
async fn main() {
    let mut redis_connection = get_redis_connection();
    match &redis_connection {
        Ok(_con) => {
            println!("Connected to redis");
        }
        Err(e) => {
            println!("Error connecting to redis : {:?}", e);
            return;
        }
    }
    let _: String = redis::cmd("FLUSHALL")
        .query(&mut redis_connection.as_mut().unwrap())
        .unwrap();
    let connecting_vec: Vec<u64> = vec![];
    let connected_vec: Vec<u64> = vec![];

    let connecting_ser = serde_json::to_string(&connecting_vec).unwrap();
    let connected_ser = serde_json::to_string(&connected_vec).unwrap();
    let _: Result<String, redis::RedisError> = redis_connection
        .as_mut()
        .unwrap()
        .set("connecting", connecting_ser);

    let _: Result<String, redis::RedisError> = redis_connection
        .as_mut()
        .unwrap()
        .set("connected", connected_ser);
    let _: Result<String, redis::RedisError> = redis_connection.as_mut().unwrap().set("con", "1");

    drop(redis_connection);
    // Configure the client with your Discord bot token in the environment.
    let token = std::env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILDS;

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
