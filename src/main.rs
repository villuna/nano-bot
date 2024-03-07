use commands::action::ActionCommandData;
use serenity::all::Interaction;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use utils::SharedStopwatch;
use std::fs;
use std::sync::Arc;
use tracing::{error, info, instrument, span, Instrument, Level};

#[cfg(debug_assertions)]
use serenity::all::GuildId;

#[cfg(not(debug_assertions))]
use serenity::all::Command;

mod commands;
#[cfg(test)]
mod test;
mod utils;


#[derive(Debug)]
struct Handler {
    actions: Vec<ActionCommandData>,
    // Keep track of how long it's been since the bot was interacted with
    // to make responses to "good bot" seem a bit more normal
    last_interaction: SharedStopwatch,
}

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
            self.last_interaction.set_now().await;
        }

        if msg.content.to_lowercase().trim_matches(|c: char| !c.is_ascii_alphabetic()) == "good bot" {
            if self.last_interaction.get().await.is_some_and(|t| t.elapsed().as_secs() < 60) {
                self.last_interaction.reset().await;

                if let Err(e) = msg.channel_id.say(&ctx.http, "I'm not a robot! But thank you.").await {
                    error!("couldn't send thank you message: {e}");
                }
            }
        }
    }

    #[instrument(skip_all)]
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        cfg_if::cfg_if! {
            if #[cfg(debug_assertions)] {
                // Register the commands in my test server if this is a debug run
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
                            commands::action::register(&self.actions),
                        ],
                    )
                    .await;

                if let Err(e) = commands {
                    error!("error creating server test commands: {e}");
                }
            } else {
                // If this is a release run, register them globally
                for cmd in [commands::say_hi::register(), commands::action::register(&self.actions)] {
                    if let Err(e) = Command::create_global_command(&ctx.http, cmd).await {
                        error!("error registering global command: {e}");
                    }
                }
            }
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
                    "action" => commands::action::run(ctx.clone(), &cmd, &self.actions).await,
                    _ => {}
                }

                self.last_interaction.set_now().await;
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

    let actions =
        serde_yaml::from_str(&fs::read_to_string("assets/actions.yaml").unwrap()).unwrap();

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler { actions, last_interaction: SharedStopwatch::new() })
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
