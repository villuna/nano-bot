use serenity::prelude::*;
use std::fs;
use std::sync::Arc;
use tracing::{error, info};

mod commands;
mod event_handler;
mod utils;

#[cfg(test)]
mod test;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Bot authorisation stuff
    let token = fs::read_to_string("token.txt").expect("couldnt read token.txt");
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    // Set up the framework with our event handler
    let mut client = Client::builder(&token, intents)
        .event_handler(event_handler::Handler::new())
        .await
        .expect("Error creating handler");

    // Make sure that Ctrl+C gracefully shuts down the bot
    let shard_manager = Arc::clone(&client.shard_manager);

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register interrupt handler");
        info!("interrupt signal recieved, shutting down");
        shard_manager.shutdown_all().await;
    });

    // Run the bot
    if let Err(why) = client.start().await {
        error!("Error starting client: {why}");
    }
}
