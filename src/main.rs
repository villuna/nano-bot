use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use std::fs;
use tracing::{error, info, instrument};

#[derive(Debug)]
struct Handler;

#[async_trait]
impl EventHandler for Handler {
    // This instrument macro is just for logging
    // it allows the log to contain some info about the message
    #[instrument(
        skip_all,
        fields(
            author = format!("{}", msg.author),
            guild = format!("{:?}", msg.guild_id)
        )
    )]
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content.starts_with("->melo") {
            info!("recieved melo");
            if let Err(e) = msg.react(&ctx.http, '🍈').await {
                error!("couldnt react to melo message: {e}");
            }
        }

        if msg.mentions_me(&ctx.http).await.unwrap_or(false) {
            info!("recieved ping");
            msg.reply_ping(&ctx.http, "Pong!")
                .await
                .expect("couldnt respond to ping");
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
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

    if let Err(why) = client.start().await {
        error!("Error starting client: {why}");
    }
}
