// TODO: Refactor this. It's a bit of a mishmash of broken stuff patched over
// not the nicest code
use std::time::Duration;

use serenity::{
    all::{
        CommandInteraction, CommandOptionType, CreateButton, CreateEmbedFooter,
        EditInteractionResponse, ResolvedOption, ResolvedValue,
    },
    builder::{
        CreateCommand, CreateCommandOption, CreateEmbed, CreateEmbedAuthor,
        CreateInteractionResponse, CreateInteractionResponseMessage,
    },
    prelude::Context,
};
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::utils::get_nano_icon;
use crate::event_handler::Handler;

use super::{create_command_fn, CommandDetails};

const PAGE_LENGTH: usize = 15;

#[derive(Debug, Clone)]
pub struct HelpDetails {
    pub name: String,
    pub details: String,
    pub sub_commands: Vec<HelpDetails>,
}
impl Default for HelpDetails {
    fn default() -> Self {
        Self {
            name: "default command".to_string(),
            details: "default text".to_string(),
            sub_commands: Vec::new(),
        }
    }
}

struct Buttons {
    back: CreateButton,
    accept: CreateButton,
    forward: CreateButton,
}

fn create_buttons(commands: usize, page: usize) -> Buttons {
    let total_pages = (commands as f32 / PAGE_LENGTH as f32).ceil() as usize;
    let mut back = CreateButton::new("back").label("←");
    let accept = CreateButton::new("accept").label("✓");
    let mut forward = CreateButton::new("forward").label("→");

    if page == 0 {
        back = back.disabled(true)
    }

    if page >= total_pages - 1 {
        forward = forward.disabled(true);
    }

    Buttons {
        back,
        accept,
        forward,
    }
}

async fn create_subcommand_help_message(
    ctx: &Context,
    details: &HelpDetails,
    page: usize,
) -> (CreateEmbed, Buttons) {
    let mut embed = CreateEmbed::new()
        .author(
            CreateEmbedAuthor::new("Nano (not a bot)")
                .icon_url(get_nano_icon(ctx).await)
                .url("https://github.com/villuna/nano-bot"),
        )
        .title(format!("Subcommands for {}:", details.name));

    embed = embed.field(
        "",
        format!("**{}: {}**", details.name, details.details),
        false,
    );
    // vertical spacing
    embed = embed.field("", "", false);
    embed = add_embed_page(embed, &details.sub_commands, Some(&details.name), page);
    let buttons = create_buttons(details.sub_commands.len(), page);
    (embed, buttons)
}

async fn create_help_message(
    ctx: &Context,
    details: &[HelpDetails],
    page: usize,
) -> (CreateEmbed, Buttons) {
    let mut embed = CreateEmbed::new()
        .author(
            CreateEmbedAuthor::new("Nano (not a bot)")
                .icon_url(get_nano_icon(ctx).await)
                .url("https://github.com/villuna/nano-bot"),
        )
        .title("Here are all the commands I can perform:");

    embed = add_embed_page(embed, details, None, page);
    let buttons = create_buttons(details.len(), page);
    (embed, buttons)
}

fn add_embed_page(
    mut embed: CreateEmbed,
    details: &[HelpDetails],
    prefix: Option<&str>,
    page: usize,
) -> CreateEmbed {
    let prefix = prefix.map(|s| format!("{s} ")).unwrap_or_default();

    for command in details.iter().skip(PAGE_LENGTH * page).take(PAGE_LENGTH) {
        let subcommand_text = (!command.sub_commands.is_empty())
            .then(|| format!(" `(see /help {})`", command.name))
            .unwrap_or("".to_owned());

        embed = embed.field(
            "",
            format!(
                "/**{}{}**: {}{}",
                prefix, command.name, command.details, subcommand_text
            ),
            false,
        );
    }

    let total_pages = (details.len() as f32 / PAGE_LENGTH as f32).ceil();
    embed = embed.footer(CreateEmbedFooter::new(format!(
        "Page {}/{}",
        page + 1,
        total_pages
    )));
    embed
}

async fn create_message(
    ctx: &Context,
    data: &[HelpDetails],
    options: &HelpCommandOptions,
    page: usize,
) -> Result<(CreateEmbed, Buttons), String> {
    match options {
        HelpCommandOptions::AllCommands => Ok(create_help_message(ctx, data, page).await),

        HelpCommandOptions::SubCommand(name) => {
            match data.iter().find(|deets| &deets.name == name) {
                None => Err(name.to_owned()),

                Some(
                    details @ HelpDetails {
                        name, sub_commands, ..
                    },
                ) => {
                    if sub_commands.is_empty() {
                        Err(name.to_owned())
                    } else {
                        Ok(create_subcommand_help_message(ctx, details, page).await)
                    }
                }
            }
        }
    }
}

enum HelpCommandOptions {
    AllCommands,
    SubCommand(String),
}

fn page_count(data: &[HelpDetails], options: &HelpCommandOptions) -> Option<usize> {
    let commands = match options {
        HelpCommandOptions::AllCommands => Some(data.len()),
        HelpCommandOptions::SubCommand(name) => data
            .iter()
            .find(|deets| &deets.name == name)
            .and_then(|details| {
                if details.sub_commands.is_empty() {
                    None
                } else {
                    Some(details.sub_commands.len())
                }
            }),
    }?;

    Some((commands as f32 / PAGE_LENGTH as f32).ceil() as _)
}

impl HelpCommandOptions {
    fn parse(options: &[ResolvedOption<'_>]) -> Self {
        if options.is_empty() {
            Self::AllCommands
        } else {
            let ResolvedValue::String(name) = options[0].value else {
                // should be unreachable
                panic!("argument to help command is of incorrect type");
            };

            Self::SubCommand(name.to_owned())
        }
    }
}

pub async fn run(ctx: Context, cmd: &CommandInteraction, handler: Handler) {
    let options = cmd.data.options();
    let options = HelpCommandOptions::parse(&options);

    let send_error_message = |http, name: String| async move {
        let message = CreateInteractionResponseMessage::new()
            .content(format!(
                "Sorry! I couldn't find a help page for the command \"{name}\""
            ))
            .ephemeral(true);

        let response = CreateInteractionResponse::Message(message);

        if let Err(e) = cmd.create_response(http, response).await {
            error!("error sending error response: {e}");
        }
    };

    let mut page = 0;
    let (embed, buttons) =
        match create_message(&ctx, &handler.help_data.read().await, &options, page).await {
            Ok(res) => res,
            Err(name) => {
                send_error_message(&ctx.http, name).await;
                return;
            }
        };

    let message = CreateInteractionResponseMessage::new()
        .embed(embed)
        .button(buttons.back)
        .button(buttons.accept)
        .button(buttons.forward);

    let total_pages = page_count(&handler.help_data.read().await, &options).unwrap();

    let response = CreateInteractionResponse::Message(message);

    if let Err(e) = cmd.create_response(&ctx.http, response).await {
        error!("error sending response to help command: {e}");
        return;
    }

    let id = cmd.get_response(&ctx.http).await.unwrap().id;

    // Create a channel through which the event handler will send button events
    let (tx, mut rx) = mpsc::channel(256);
    handler.button_event_tx.write().await.insert(id, tx);

    // This future loops forever, recieving button interactions and updating the help message
    // accordingly. We use a timeout later to cap the running time of this loop.
    let recv_loop = async {
        'mainloop: loop {
            match rx.recv().await {
                None => {
                    error!("channel closed unexpectedly!");
                    break 'mainloop;
                }

                Some(interaction) if interaction.user.id == cmd.user.id => {
                    info!("recieved button interaction");
                    match interaction.data.custom_id.as_str() {
                        "back" => {
                            page = page.saturating_sub(1);
                        }

                        "forward" => {
                            if page < total_pages {
                                page += 1;
                            }
                        }

                        "accept" => {
                            break 'mainloop;
                        }

                        s => {
                            error!("invalid button event recieved: \"{}\"! did you forget to handle it?", s);
                        }
                    }

                    let (embed, buttons) =
                        match create_message(&ctx, &handler.help_data.read().await, &options, page)
                            .await
                        {
                            Ok(res) => res,
                            Err(name) => {
                                send_error_message(&ctx.http, name).await;
                                return;
                            }
                        };

                    let response = EditInteractionResponse::new()
                        .embed(embed)
                        .button(buttons.back)
                        .button(buttons.accept)
                        .button(buttons.forward);

                    if let Err(e) = cmd.edit_response(&ctx.http, response).await {
                        error!("error sending response to help command: {e}");
                        return;
                    }

                    let acknowledge = CreateInteractionResponse::Acknowledge;
                    if let Err(e) = interaction.create_response(&ctx.http, acknowledge).await {
                        error!("error acknowledging button interaction: {e}");
                        return;
                    }
                }

                Some(interaction) => {
                    let acknowledge = CreateInteractionResponse::Acknowledge;
                    if let Err(e) = interaction.create_response(&ctx.http, acknowledge).await {
                        error!("error acknowledging button interaction from wrong user: {e}");
                        return;
                    }
                }
            }
        }
    };

    // Create a timer and only run the previous loop until the timer dings
    let timeout = tokio::time::sleep(Duration::from_secs(60));
    tokio::select! {
        _ = timeout => {},
        _ = recv_loop => {},
    }

    // Now we can remove the channel and update the message to have no buttons.
    info!("shutting down handler thread for help message");
    handler.button_event_tx.write().await.remove(&id);

    let (embed, _) =
        match create_message(&ctx, &handler.help_data.read().await, &options, page).await {
            Ok(res) => res,
            Err(name) => {
                send_error_message(&ctx.http, name).await;
                return;
            }
        };

    let response = EditInteractionResponse::new()
        .components(Vec::new())
        .embed(embed);

    if let Err(e) = cmd.edit_response(&ctx.http, response).await {
        error!("error updating help command: {e}");
    }
}

pub fn register() -> CommandDetails {
    let registration = CreateCommand::new("help")
        .description("A list of the commands that can be used")
        .add_option(CreateCommandOption::new(
            CommandOptionType::String,
            "command",
            "an (optional) command to get detailed info about",
        ));

    let help = HelpDetails {
        name: "help".to_string(),
        details:
            "Get help on what commands can be used. You can also query information on commands that have sub commands."
                .to_string(),
        sub_commands: Vec::new(),
    };

    let command =
        create_command_fn(|ctx, handler, cmd| async move { run(ctx, &cmd, handler).await });

    CommandDetails {
        name: "help".to_owned(),
        registration,
        help,
        command,
    }
}
