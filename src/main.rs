use serenity::all::Interaction;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use serenity::{all::GuildId, async_trait};
use std::fs;
use std::sync::Arc;
use tracing::{error, info, instrument, span, Instrument, Level};

mod commands;

#[derive(Debug)]
struct Handler;

#[async_trait]
impl EventHandler for Handler {
    // This instrument macro is just for logging
    // it allows the log to contain some info about the message
    #[instrument(
        skip_all,
        fields(
            author = %msg.author,
            guild = ?msg.guild_id,
        )
    )]
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content.starts_with("->melo") {
            info!("recieved melo");
            if let Err(e) = msg.react(&ctx.http, '🍈').await {
                error!("couldnt react to melo message: {e}");
            }
        }
    }

    #[instrument(skip_all)]
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        let test_guild_id = GuildId::new(
            fs::read_to_string("test_guild_id.txt")
                .expect("couldnt read test guild id")
                .trim()
                .parse()
                .expect("guild id must be an integer"),
        );

        let commands = test_guild_id
            .set_commands(
                &ctx.http, 
                vec![
                    commands::say_hi::register(),
                ]
            )
            .await;

        if let Err(e) = commands {
            error!("error creating commands: {e}");
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(cmd) = interaction {
            let span = span!(
                Level::INFO, "command", 
                user = cmd.user.name,
                guild = ?cmd.guild_id,
                cmd = cmd.data.name, 
                options = ?cmd.data.options
            );

            async move {
                info!("recieved command");
                match cmd.data.name.as_str() {
                    "sayhi" => commands::say_hi::run(ctx.clone(), &cmd).await,
                    _ => {}
                }
            }
            .instrument(span)
            .await;
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let token = fs::read_to_string("token.txt").expect("couldnt read token.txt");

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Error creating handler");

    let shard_manager = Arc::clone(&client.shard_manager);

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register interrupt handler");
        info!("interrupt signal recieved, shutting down");
        shard_manager.shutdown_all().await;
    });

    if let Err(why) = client.start().await {
        error!("Error starting client: {why}");
    }
}
