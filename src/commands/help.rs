use serenity::{
    all::{CommandInteraction, CommandOptionType, ResolvedValue},
    builder::{
        CreateCommand, CreateCommandOption, CreateEmbed, CreateEmbedAuthor,
        CreateInteractionResponse, CreateInteractionResponseMessage,
    },
    prelude::Context,
};
use tracing::error;

use crate::utils::get_nano_icon;

use super::{create_command_fn, CommandDetails};

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

async fn create_subcommand_help_embed(ctx: &Context, details: &HelpDetails) -> CreateEmbed {
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
    embed = add_embed_page(embed, &details.sub_commands, Some(&details.name));

    embed
}

async fn create_help_embed(ctx: &Context, details: &[HelpDetails]) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .author(
            CreateEmbedAuthor::new("Nano (not a bot)")
                .icon_url(get_nano_icon(ctx).await)
                .url("https://github.com/villuna/nano-bot"),
        )
        .title("Here are all the commands I can perform:");

    embed = add_embed_page(embed, details, None);
    embed
}

fn add_embed_page(
    mut embed: CreateEmbed,
    details: &[HelpDetails],
    prefix: Option<&str>,
) -> CreateEmbed {
    let prefix = prefix.map(|s| format!("{s} ")).unwrap_or_default();

    for command in details {
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

    embed
}

pub async fn run(ctx: Context, cmd: &CommandInteraction, data: &[HelpDetails]) {
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

    let embed = if options.is_empty() {
        create_help_embed(&ctx, data).await
    } else {
        let ResolvedValue::String(name) = options[0].value else {
            error!("argument to help command is of incorrect type");
            return;
        };

        match data.iter().find(|deets| deets.name == name) {
            None => {
                send_error_message(&ctx.http, name.to_owned()).await;
                return;
            }

            Some(
                details @ HelpDetails {
                    name, sub_commands, ..
                },
            ) => {
                if sub_commands.is_empty() {
                    send_error_message(&ctx.http, name.to_owned()).await;
                    return;
                }

                create_subcommand_help_embed(&ctx, details).await
            }
        }
    };

    let message = CreateInteractionResponseMessage::new().embed(embed);
    let response = CreateInteractionResponse::Message(message);

    if let Err(e) = cmd.create_response(&ctx.http, response).await {
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
            "Get help on what commands can be used. You can also query information on sub commands."
                .to_string(),
        sub_commands: Vec::new(),
    };

    let command = create_command_fn(|ctx, handler, cmd| async move {
        run(ctx, &cmd, &handler.help_data.read().await).await
    });

    CommandDetails {
        name: "help".to_owned(),
        registration,
        help,
        command,
    }
}
