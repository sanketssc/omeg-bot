// use std::env;

mod commands;

use serde::{Deserialize, Serialize};
use serenity::all::{
    ActivityData, ButtonStyle, ChannelId, ChannelType, Command, CommandInteraction,
    CreateAttachment, CreateButton, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage, CreateThread, EditInteractionResponse, Guild, GuildChannel, Interaction,
    Message, PartialGuildChannel, ResolvedValue, UserId,
};
use serenity::async_trait;
use serenity::futures::StreamExt;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

use redis::Commands;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct User {
    id: UserId,
    channel: ChannelId,
    interests: Vec<String>,
    partner: Option<UserId>,
    partner_channel: Option<ChannelId>,
}

struct Handler;

#[derive(Debug)]
enum GenericError {
    RedisError(redis::RedisError),
    SerenityError(serenity::Error),
    // SerenityJsonError(serenity::model::),
    SerdeJsonError(serde_json::Error),
}

impl From<redis::RedisError> for GenericError {
    fn from(error: redis::RedisError) -> Self {
        GenericError::RedisError(error)
    }
}

impl From<serenity::Error> for GenericError {
    fn from(error: serenity::Error) -> Self {
        GenericError::SerenityError(error)
    }
}

impl From<serde_json::Error> for GenericError {
    fn from(error: serde_json::Error) -> Self {
        GenericError::SerdeJsonError(error)
    }
}

async fn matcher(
    ctx: &Context,
    command: &CommandInteraction,
    redis_connection: &mut redis::Connection,
) -> Result<String, GenericError> {
    let connecting: String = redis_connection.get("connecting")?;
    let connected: String = redis_connection.get("connected")?;
    println!("Connecting: {:?}", connecting);
    println!("Connected: {:?}", connected);

    let mut connecting_vec: Vec<User> = serde_json::from_str(&connecting)?;
    println!("Connecting vec: {:?}", connecting_vec);
    let mut connected_vec: Vec<User> = serde_json::from_str(&connected)?;

    if let Some(_val) = connecting_vec.iter().find(|u| u.id == command.user.id) {
        println!("You are already in queue");

        return Ok("You are already in queue".to_string());
    }

    if let Some(_val) = connected_vec.iter().find(|u| u.id == command.user.id) {
        println!("You are already connected");
        let msg_str = format!(
            "You are already connected -> <#{}>",
            _val.channel.to_string()
        );
        return Ok(msg_str);
    }

    let thread_name = format!(
        "{}{}",
        command.user.id.to_string(),
        command.user.global_name.clone().unwrap()
    );
    let x = CreateThread::new(thread_name)
        .invitable(false)
        .kind(ChannelType::PrivateThread)
        .auto_archive_duration(serenity::all::AutoArchiveDuration::OneHour);
    let _res = ctx
        .http()
        .create_thread(command.channel_id, &x, Some("Hello"))
        .await?;

    println!("Thread created: {:?}", _res);
    ctx.http()
        .send_message(
            _res.id,
            Vec::<CreateAttachment>::new(),
            &CreateMessage::new().content("Hello"),
        )
        .await?;
    _res.id
        .add_thread_member(&ctx.http, command.user.id)
        .await?;

    // println!("interest: {:?}", command.data.options);

    println!("Interests: {:?}", command.data.options);
    let insts = match command.data.options.len() {
        0 => vec![],
        _ => match command.data.options()[0].value.clone() {
            ResolvedValue::String(interest) => interest
                .split(",")
                .map(|x| x.trim().to_string())
                .collect::<Vec<_>>(),
            _ => vec![],
        },
    };
    // println!("Interests: {:?}", insts);

    let mut user = User {
        id: command.user.id,
        channel: _res.id,
        interests: insts,
        partner: None,
        partner_channel: None,
    };
    _res.id
        .say(&ctx.http, "Waiting for user to connect")
        .await?;

    if connecting_vec.len() > 0 {
        connecting_vec.sort_by(|a, b| b.interests.len().cmp(&a.interests.len()));
        let mut free_user = connecting_vec[0].clone();
        connecting_vec.remove(0);
        free_user.partner = Some(command.user.id);
        user.partner = Some(free_user.id);
        free_user.partner_channel = Some(user.channel);
        user.partner_channel = Some(free_user.channel);
        user.channel
            .say(&ctx.http, "You are connected to user")
            .await?;
        free_user
            .channel
            .say(&ctx.http, "You are connected to user")
            .await?;

        connected_vec.push(free_user.clone());
        connected_vec.push(user.clone());
        let connected_ser = serde_json::to_string(&connected_vec)?;
        let connecting_ser = serde_json::to_string(&connecting_vec)?;
        redis_connection.set("connected", connected_ser)?;
        redis_connection.set("connecting", connecting_ser)?;
        redis_connection.set(command.user.id.to_string(), free_user.id.to_string())?;
        redis_connection.set(free_user.id.to_string(), command.user.id.to_string())?;
        redis_connection.set(free_user.channel.to_string(), user.channel.to_string())?;
        redis_connection.set(user.channel.to_string(), free_user.channel.to_string())?;

        let msg_str = format!("You are connected to user -> <#{}>", _res.id.to_string());
        return Ok(msg_str);
    }

    connecting_vec.push(user);

    // println!("Subscribed to user: {:?}", command.user.id);
    let connecting_ser = serde_json::to_string(&connecting_vec)?;
    redis_connection.set("connecting", connecting_ser)?;
    let msg_str = format!("You can chat with your Partner here -->  <#{}>", _res.id);
    Ok(msg_str)
}

async fn disconnect_users(
    user1: UserId,
    ctx: &Context,
    redis_connection: &mut redis::Connection,
) -> Result<(), GenericError> {
    let connected: String = redis_connection.get("connected")?;
    let mut connected_vec: Vec<User> = serde_json::from_str(&connected)?;

    if let Some(u) = connected_vec.iter().find(|u| u.id == user1) {
        let user2 = u.partner.unwrap();
        let user2_channel = u.partner_channel.unwrap();
        let user1_channel = u.channel;
        user2_channel.delete(&ctx.http).await?;
        user1_channel.delete(&ctx.http).await?;
        redis_connection.del(user1.to_string())?;
        redis_connection.del(user2.to_string())?;
        redis_connection.del(user1_channel.to_string())?;
        redis_connection.del(user2_channel.to_string())?;

        connected_vec.retain(|u| u.id != user1);
        connected_vec.retain(|u| u.id != user2);
        let connected_ser = serde_json::to_string(&connected_vec)?;
        redis_connection.set("connected", connected_ser)?;
    }
    Ok(())
}

async fn cancel_wait(
    ctx: &Context,
    command: &CommandInteraction,
    redis_connection: &mut redis::Connection,
) -> Result<String, GenericError> {
    let connecting: String = redis_connection.get("connecting")?;
    let mut connecting_vec: Vec<User> = serde_json::from_str(&connecting)?;
    let connected: String = redis_connection.get("connected")?;
    let connected_vec: Vec<User> = serde_json::from_str(&connected)?;

    if let Some(u) = connecting_vec.iter().find(|u| u.id == command.user.id) {
        let user1_channel = u.channel;
        user1_channel.delete(&ctx.http).await?;
        connecting_vec.retain(|u| u.id != command.user.id);
        let connecting_ser = serde_json::to_string(&connecting_vec)?;
        redis_connection.set("connecting", connecting_ser)?;
        Ok("Successfully cancelled the request".to_string())
    } else if let Some(u) = connected_vec.iter().find(|u| u.id == command.user.id) {
        println!("User not found in connecting");
        disconnect_users(u.id, ctx, redis_connection).await?;
        Ok("Successfully cancelled the request".to_string())
    } else {
        //replace /start with command id of start command
        Ok("You are not in queue. \n Use /start to connect to stranger".to_string())
    }
}

fn get_redis_connection() -> Result<redis::Connection, redis::RedisError> {
    let client = redis::Client::open(
        std::env::var("REDIS_URL").expect("REDIS_URL must be set in the environment"),
        // "redis://127.0.0.1:6379",
    )?;
    let mut con: redis::Connection = client.get_connection()?;
    let _ = con.set("con", "true")?;
    Ok(con)
}

#[derive(Serialize)]
struct MapForThread;

async fn try_interaction_create(
    ctx: Context,
    interaction: Interaction,
) -> Result<(), GenericError> {
    if let Interaction::Command(command) = interaction {
        let redis_connection = get_redis_connection();

        match &command.channel.clone().unwrap().kind {
            ChannelType::PrivateThread => {
                if command.data.name.as_str() != "leave" {
                    command
                        .create_response(
                            ctx,
                            CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new()
                                    .content("You can only use /leave command in the thread")
                                    .ephemeral(true),
                            ),
                        )
                        .await?;

                    return Ok(());
                }
            }
            _ => {}
        }

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
                    .await?;
                return Ok(());
            }
        }

        let mut redis_connection = redis_connection?;

        println!("Interaction received: {:?}", command);
        println!("from : {:?}", command.user.global_name.clone().unwrap());

        match command.data.name.as_str() {
            "pinga" => Some(commands::ping::run(&command.data.options())),
            "cancel" => {
                let res = cancel_wait(&ctx, &command, &mut redis_connection).await?;
                command
                    .create_response(
                        ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content(res)
                                .ephemeral(true),
                        ),
                    )
                    .await?;
                return Ok(());
            }
            "start" => {
                println!(
                    "Interaction received: {:?}",
                    command.user.global_name.clone().unwrap()
                );
                command.defer_ephemeral(&ctx.http).await?;
                command
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new().content("Starting Connection"),
                    )
                    .await?;

                let response = matcher(&ctx, &command, &mut redis_connection).await;

                match response {
                    Ok(msg) => {
                        let x = command
                            .edit_response(
                                &ctx.http,
                                EditInteractionResponse::new().content(msg).button(
                                    CreateButton::new("cancel")
                                        .style(ButtonStyle::Danger)
                                        .label("Cancel"),
                                ),
                            )
                            .await?;

                        let mut interaction_stream = x.await_component_interaction(&ctx).stream();

                        while let Some(interaction) = interaction_stream.next().await {
                            match interaction.data.custom_id.as_str() {
                                "cancel" => {
                                    cancel_wait(&ctx, &command, &mut redis_connection).await?;
                                    command
                                        .edit_response(
                                            &ctx.http,
                                            EditInteractionResponse::new().content("Cancelled"),
                                        )
                                        .await?;
                                    interaction
                                        .create_response(
                                            &ctx.http,
                                            CreateInteractionResponse::UpdateMessage(
                                                CreateInteractionResponseMessage::new()
                                                    .content("Successfully cancelled the request"),
                                            ),
                                        )
                                        .await?;
                                    interaction.delete_response(&ctx.http).await?;

                                    break;
                                }
                                _ => {
                                    command
                                        .edit_response(
                                            &ctx.http,
                                            EditInteractionResponse::new()
                                                .content("Unknown command"),
                                        )
                                        .await?;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("Error: {:?}", e);
                        command
                            .edit_response(
                                &ctx.http,
                                EditInteractionResponse::new().content("Some error occured"),
                            )
                            .await?;
                    }
                }
                Some("Ok".to_string())
            }
            "leave" => {
                disconnect_users(command.user.id, &ctx, &mut redis_connection).await?;

                Some("Ok".to_string())
            }
            _ => Some("Unknown command".to_string()),
        };
    }
    return Ok(());
}

async fn redis_delete(
    key: &str,
    redis_connection: &mut redis::Connection,
) -> Result<(), GenericError> {
    let _: () = redis_connection.del(key)?;
    Ok(())
}

async fn redis_set(
    key: &str,
    value: &str,
    redis_connection: &mut redis::Connection,
) -> Result<(), GenericError> {
    let _: () = redis_connection.set(key, value)?;
    Ok(())
}
#[async_trait]
impl EventHandler for Handler {
    // async fn channel_delete(
    //     &self,
    //     ctx: Context,
    //     channel: GuildChannel,
    //     messages: Option<Vec<Message>>,
    // ) {
    //     println!("Channel deleted: {:?}", channel.id);
    //     // let redis_connection = get_redis_connection().unwrap();
    //     // let connected: String = redis_connection.get("connected").unwrap();
    //     // let mut connected_vec: Vec<User> = serde_json::from_str(&connected).unwrap();
    //     // connected_vec.retain(|u| u.channel != channel);
    //     // let connected_ser = serde_json::to_string(&connected_vec).unwrap();
    //     // redis_connection.set("connected", connected_ser).unwrap();
    // }
    async fn thread_delete(
        &self,
        ctx: Context,
        partial_channel: PartialGuildChannel,
        _channel: Option<GuildChannel>,
    ) {
        println!("Thread deleted: {:?}", partial_channel.id);
        let mut redis_connection = get_redis_connection().unwrap();
        let connected: String = redis_connection.get("connected").unwrap();
        let connecting: String = redis_connection.get("connecting").unwrap();

        let mut connecting_vec: Vec<User> = serde_json::from_str(&connecting).unwrap();

        let mut connected_vec: Vec<User> = serde_json::from_str(&connected).unwrap();
        let user = if let Some(user) = connected_vec
            .iter()
            .find(|u| u.channel == partial_channel.id)
        {
            user.clone()
        } else {
            if let Some(user) = connecting_vec
                .clone()
                .iter()
                .find(|u| u.channel == partial_channel.id)
            {
                connecting_vec.retain(|u| u.channel != user.channel);
                let connecting_ser = serde_json::to_string(&connecting_vec).unwrap();
                redis_set("connecting", &connecting_ser, &mut redis_connection)
                    .await
                    .unwrap();
                return;
            } else {
                return;
            };
        };
        let thread_id = user.channel;
        let partner_thread_id = user.partner_channel.unwrap();

        let res = ctx
            .http()
            .delete_channel(partner_thread_id, Some("Partner Left the chat"))
            .await;
        match res {
            Ok(_) => {
                println!("Thread deleted successfully");
            }
            Err(_) => {
                // println!("Error deleting thread: {:?}", e);
                return;
            }
        }
        redis_delete(&user.id.to_string(), &mut redis_connection)
            .await
            .unwrap();
        redis_delete(&user.partner.unwrap().to_string(), &mut redis_connection)
            .await
            .unwrap();
        redis_delete(&thread_id.to_string(), &mut redis_connection)
            .await
            .unwrap();
        redis_delete(&partner_thread_id.to_string(), &mut redis_connection)
            .await
            .unwrap();

        connected_vec.retain(|u| u.channel != partner_thread_id);
        connected_vec.retain(|u| u.channel != thread_id);

        let connected_ser = serde_json::to_string(&connected_vec).unwrap();
        redis_set("connected", &connected_ser, &mut redis_connection)
            .await
            .unwrap();
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let _res = try_interaction_create(ctx, interaction).await;
    }
    async fn guild_create(&self, _ctx: Context, guild: Guild, is_new: Option<bool>) {
        if is_new.unwrap() {
            println!("Bot joined a new guild:");
            println!("Guild Name: {}", guild.name);
            println!("Guild ID: {}", guild.id);
            println!("---------------------");
        }
    }
    async fn message(&self, ctx: Context, msg: Message) {
        let is_bot = msg.author.bot;
        if is_bot {
            return;
        }
        let thread_name = format!(
            "{}{}",
            msg.author.id.to_string(),
            msg.author.global_name.clone().unwrap()
        );
        let name = msg.channel_id.name(&ctx.http).await.unwrap();

        if thread_name != name {
            return;
        }

        let chan_id = msg.channel_id;

        let atch = &msg.attachments;
        if atch.len() > 0 {
            msg.delete(&ctx.http).await.unwrap();
            chan_id
                .say(
                    &ctx.http,
                    "Attachments are not allowed\n Premium comming soon!~",
                )
                .await
                .unwrap();
            return;
        }
        let stckr = &msg.sticker_items;
        if stckr.len() > 0 {
            msg.delete(&ctx.http).await.unwrap();
            chan_id
                .say(
                    &ctx.http,
                    "Stickers are not allowed\n Premium comming soon!~",
                )
                .await
                .unwrap();
            return;
        }

        let cha = msg.channel(&ctx.http).await.unwrap();
        let kind = cha.guild().unwrap().kind;
        match kind {
            ChannelType::PrivateThread => {
                println!("Thread name:  {}", thread_name);

                let target_chan: Result<String, redis::RedisError> =
                    get_redis_connection().unwrap().get(chan_id.to_string());

                match target_chan {
                    Ok(target_chan) => {
                        println!("Target channel: {:?}", target_chan);
                        let target_chan_id = ChannelId::from(target_chan.parse::<u64>().unwrap());
                        target_chan_id.say(&ctx.http, msg.content).await.unwrap();
                    }
                    Err(e) => {
                        println!("Error: {:?}", e);
                        chan_id
                            .say(
                                &ctx.http,
                                "You are not connected to anyone Please wait until someone connect",
                            )
                            .await
                            .unwrap();
                        msg.delete(&ctx.http).await.unwrap();
                    }
                };
            }
            _ => {}
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

        Command::create_global_command(&ctx.http, commands::start::register())
            .await
            .unwrap();

        Command::create_global_command(&ctx.http, commands::leave::register())
            .await
            .unwrap();

        Command::create_global_command(&ctx.http, commands::cancel::register())
            .await
            .unwrap();

        // println!("I created the following global slash command: {c1:#?} {c2:#?} {c3:#?}");
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
        | GatewayIntents::GUILDS
        | GatewayIntents::all();

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
