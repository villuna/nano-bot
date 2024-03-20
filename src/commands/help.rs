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

use crate::{event_handler::HandlerInner, utils::get_nano_icon};

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

fn create_buttons(commands: usize, page: usize) -> (CreateButton, CreateButton) {
    let total_pages = (commands as f32 / PAGE_LENGTH as f32).ceil() as usize;
    let mut back = CreateButton::new("back").label("←");
    let mut forward = CreateButton::new("forward").label("→");

    if page == 0 {
        back = back.disabled(true)
    }

    if page >= total_pages - 1 {
        forward = forward.disabled(true);
    }

    (back, forward)
}

async fn create_subcommand_help_message(
    ctx: &Context,
    details: &HelpDetails,
    page: usize,
) -> (CreateEmbed, CreateButton, CreateButton) {
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
    let (back, forward) = create_buttons(details.sub_commands.len(), page);
    (embed, back, forward)
}

async fn create_help_message(
    ctx: &Context,
    details: &[HelpDetails],
    page: usize,
) -> (CreateEmbed, CreateButton, CreateButton) {
    let mut embed = CreateEmbed::new()
        .author(
            CreateEmbedAuthor::new("Nano (not a bot)")
                .icon_url(get_nano_icon(ctx).await)
                .url("https://github.com/villuna/nano-bot"),
        )
        .title("Here are all the commands I can perform:");

    embed = add_embed_page(embed, details, None, page);
    let (back, forward) = create_buttons(details.len(), page);
    (embed, back, forward)
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
    options: &[ResolvedOption<'_>],
    page: usize,
) -> Result<(CreateEmbed, CreateButton, CreateButton), String> {
    if options.is_empty() {
        Ok(create_help_message(ctx, data, page).await)
    } else {
        let ResolvedValue::String(name) = options[0].value else {
            // should be unreachable
            panic!("argument to help command is of incorrect type");
        };

        match data.iter().find(|deets| deets.name == name) {
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

pub async fn run(ctx: Context, cmd: &CommandInteraction, handler: &HandlerInner) {
    let options = cmd.data.options();

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
    let (embed, back, forward) =
        match create_message(&ctx, &handler.help_data.read().await, &options, page).await {
            Ok(res) => res,
            Err(name) => {
                send_error_message(&ctx.http, name).await;
                return;
            }
        };

    let message = CreateInteractionResponseMessage::new()
        .embed(embed)
        .button(back)
        .button(forward);

    let response = CreateInteractionResponse::Message(message);

    if let Err(e) = cmd.create_response(&ctx.http, response).await {
        error!("error sending response to help command: {e}");
        return;
    }

    let id = cmd.get_response(&ctx.http).await.unwrap().id;

    // Create a channel through which the event handler will send button events
    let (tx, mut rx) = mpsc::channel(256);
    handler.button_event_tx.write().await.insert(id, tx);

    'mainloop: loop {
        let timeout = tokio::time::sleep(Duration::from_secs(15));

        tokio::select! {
            _ = timeout => break 'mainloop,
            res = rx.recv() => match res {
                None => {
                    error!("channel closed unexpectedly!");
                    break 'mainloop;
                },
                Some(interaction) => {
                    info!("recieved button interaction");
                    if interaction.data.custom_id == "back" {
                        page -= 1;
                    } else if interaction.data.custom_id == "forward" {
                        page += 1;
                    } else {
                        unreachable!();
                    }

                    let (embed, back, forward) = match create_message(&ctx, &handler.help_data.read().await, &options, page).await {
                        Ok(res) => res,
                        Err(name) => {
                            send_error_message(&ctx.http, name).await;
                            return;
                        }
                    };

                    let response = EditInteractionResponse::new()
                        .embed(embed)
                        .button(back)
                        .button(forward);

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
            },
        }
    }

    handler.button_event_tx.write().await.remove(&id);

    info!("shutting down handler thread for help message");
    // At this point we want to prevent the user from trying to change the page
    // so we will recreate the help message without the buttons
    let (embed, _, _) =
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
        error!("error sending response to help command: {e}");
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
        create_command_fn(|ctx, handler, cmd| async move { run(ctx, &cmd, &handler).await });

    CommandDetails {
        name: "help".to_owned(),
        registration,
        help,
        command,
    }
}
