use crate::commands::say_hi::SayHiData;
use crate::commands::{self, CommandFn};
use crate::commands::{action::ActionCommandData, help::HelpDetails};
use crate::utils::SharedStopwatch;
use serenity::all::{ComponentInteraction, ComponentInteractionDataKind, Interaction, MessageId};
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;
use tracing::{error, info, instrument, span, Instrument, Level};

#[cfg(debug_assertions)]
use serenity::all::GuildId;

#[cfg(not(debug_assertions))]
use serenity::all::Command;

#[derive(Clone)]
pub struct Handler {
    inner: Arc<HandlerInner>,
}

impl Handler {
    pub fn new() -> Handler {
        Self {
            inner: Arc::new(HandlerInner::new()),
        }
    }
}

impl Deref for Handler {
    type Target = HandlerInner;

    fn deref(&self) -> &HandlerInner {
        &self.inner
    }
}

#[derive(Default)]
pub struct HandlerInner {
    pub http_client: reqwest::Client,
    commands: RwLock<HashMap<String, CommandFn>>,

    // A (static) list of all the data associated with action commands
    // read from assets/actions.yaml
    pub actions: Vec<ActionCommandData>,
    pub say_hi_data: Vec<SayHiData>,
    pub help_data: RwLock<Vec<HelpDetails>>,
    // Keep track of how long it's been since the bot was interacted with
    // to make responses to "good bot" seem a bit more normal
    last_interaction: SharedStopwatch,

    pub button_event_tx: RwLock<HashMap<MessageId, mpsc::Sender<ComponentInteraction>>>,
}

impl HandlerInner {
    pub fn new() -> Self {
        let actions =
            serde_yaml::from_str(&fs::read_to_string("assets/actions.yaml").unwrap()).unwrap();
        let say_hi_data =
            serde_yaml::from_str(&fs::read_to_string("assets/say_hi.yaml").unwrap()).unwrap();

        Self {
            actions,
            say_hi_data,
            ..Default::default()
        }
    }
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
            self.inner.last_interaction.set_now().await;
        }

        if self
            .inner
            .last_interaction
            .get()
            .await
            .is_some_and(|t| t.elapsed().as_secs() < 60)
        {
            // Respond to "good/bad bot" messages if nano has done something within the last
            // 60 seconds
            let is_good_bot = msg
                .content
                .to_lowercase()
                .trim_matches(|c: char| !c.is_ascii_alphabetic())
                == "good bot";

            let is_bad_bot = msg
                .content
                .to_lowercase()
                .trim_matches(|c: char| !c.is_ascii_alphabetic())
                == "bad bot";

            if is_good_bot {
                // Once she responds to a good/bad bot message, she probably shouldnt respond to
                // another until she does some other helpful thing
                // so reset the stopwatch
                self.inner.last_interaction.unset().await;

                if let Err(e) = msg
                    .channel_id
                    .say(&ctx.http, "I'm not a robot! But thank you.")
                    .await
                {
                    error!("couldn't send thank you message: {e}");
                }
            } else if is_bad_bot {
                self.inner.last_interaction.unset().await;

                let gif_url =
                    "https://media1.tenor.com/m/02kmUuBVE9IAAAAd/watch-yo-tone-nichijou.gif";

                if let Err(e) = msg.reply_ping(&ctx.http, gif_url).await {
                    error!("couldnt slap user: {e}");
                }
            }
        }
    }

    #[instrument(skip_all)]
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        // Registering commands
        // For debug runs, the commands will just be registered in my test server as server-scoped
        // commands update instantly. For release runs, this will be global so that the commands
        // may be used anywhere.

        let mut commands = vec![commands::help::register(), commands::say_hi::register()];

        commands.extend(commands::action::register(&self.inner.actions));

        *self.inner.help_data.write().await = commands.iter().map(|cmd| cmd.help.clone()).collect();

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

                let register_result = test_guild_id
                    .set_commands(
                        ctx,
                        commands.iter().map(|cmd| cmd.registration.clone()).collect(),
                    )
                    .await;

                if let Err(e) = register_result {
                    error!("error creating server test commands: {e}");
                }
            } else {
                // If this is a release run, register them globally
                let register_result =
                    Command::set_global_commands(
                        ctx,
                        commands.iter().map(|cmd| cmd.registration.clone()).collect(),
                    )
                    .await;

                if let Err(e) = register_result {
                    error!("error creating global commands: {e}");
                }
            }
        }

        for cmd in commands {
            info!("registered command \"{}\"", cmd.name);
            self.inner
                .commands
                .write()
                .await
                .insert(cmd.name, cmd.command);
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(cmd) => {
                // just for logging purposes
                // makes our logs display information about the command
                // (user, guild, command name, options)
                let span = span!(
                    Level::INFO, "command",
                    user = cmd.user.name,
                    guild = ?cmd.guild_id,
                    cmd = cmd.data.name,
                    options = ?cmd.data.options
                );

                // the whole async move - instrument - await thing is also just for logging purposes
                async move {
                    info!("recieved command");
                    self.inner.last_interaction.set_now().await;

                    let name = cmd.data.name.as_str();

                    match self.inner.commands.read().await.get(name) {
                        Some(command) => command(ctx.clone(), self.clone(), cmd).await,
                        None => error!("Command is unrecognised: {name}"),
                    }
                }
                .instrument(span)
                .await;
            }

            Interaction::Component(interaction) => {
                let span = span!(
                    Level::INFO, "command",
                    user = interaction.user.name,
                    guild = ?interaction.guild_id,
                    id = interaction.data.custom_id,
                );

                async move {
                    if matches!(interaction.data.kind, ComponentInteractionDataKind::Button) {
                        info!("recieved button interaction. transmitting it to handler thread");
                        let txs = self.inner.button_event_tx.read().await;
                        if let Some(tx) = txs.get(&interaction.message.id) {
                            if let Err(SendError(ev)) = tx.send(interaction.clone()).await {
                                error!(
                                    "couldn't send button event ({}) to reciever",
                                    ev.data.custom_id
                                );
                                drop(txs);
                                let mut txs = self.inner.button_event_tx.write().await;
                                txs.remove(&interaction.message.id);
                            }
                        }
                    }
                }
                .instrument(span)
                .await;
            }

            _ => {}
        }
    }
}
