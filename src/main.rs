use serenity::prelude::*;
use tokio::signal::unix::SignalKind;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::Rotation;
use std::{fs, io};
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::fmt;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tokio::signal;

mod commands;
mod event_handler;
mod utils;

#[cfg(test)]
mod test;

fn setup_logging() -> Result<WorkerGuard, Box<dyn std::error::Error>> {
    std::fs::create_dir_all("./log")?;

    let file_appender = tracing_appender::rolling::Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix("nano")
        .filename_suffix("log")
        .max_log_files(10)
        .build("./log")?;

    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with(fmt::Layer::new().with_writer(io::stdout))
        .with(fmt::Layer::new().with_writer(non_blocking));
    tracing::subscriber::set_global_default(subscriber).expect("unable to set a global subscriber");

    Ok(guard)
}

#[tokio::main]
async fn main() {
    let _guard = match setup_logging() {
        Ok(guard) => {
            Some(guard)
        },
        Err(e) => {
            fmt::init();
            // It does occur to me that this error message will be lost forever
            // but maybe it will be important
            error!("error setting up log file logging: {e}");
            None
        },
    };

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
        #[cfg(target_family = "unix")]
        let shutdown_signal = async {
            let ctrl_c = signal::ctrl_c();
            let mut sigterm = signal::unix::signal(SignalKind::terminate())
                .expect("couldn't register sigterm handler");

            tokio::select! {
                res1 = ctrl_c => res1.expect("couldnt register ctrl_c handler"),
                res2 = sigterm.recv() => {
                    if res2.is_none() {
                        error!("sigterm recv error");
                    }
                },
            };
        };

        #[cfg(not(target_family = "unix"))]
        let shutdown_signal = async {
            signal::ctrl_c()
                .await
                .expect("couldnt register ctrl_c handler");
        };

        shutdown_signal.await;

        info!("interrupt signal recieved, shutting down");
        shard_manager.shutdown_all().await;
    });

    // Run the bot
    if let Err(why) = client.start().await {
        error!("Error starting client: {why}");
    }
}
