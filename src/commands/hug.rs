// Prototype image command. Will be used as a template, maybe for a macro or some other smart way
// of creating image commands in the future

use serde::Deserialize;
use serenity::{
    all::{CommandInteraction, CommandOptionType, ResolvedValue, User},
    builder::{
        CreateCommand, CreateCommandOption, CreateEmbed, CreateInteractionResponse,
        CreateInteractionResponseMessage,
    },
    prelude::Context,
    utils::MessageBuilder,
};
use tracing::error;

#[derive(Deserialize)]
struct ImageResponse {
    url: String,
}

async fn get_image() -> reqwest::Result<String> {
    let response = reqwest::get("https://api.otakugifs.xyz/gif?reaction=hug")
        .await?
        .json::<ImageResponse>()
        .await?;

    Ok(response.url)
}

pub async fn run(ctx: Context, cmd: &CommandInteraction) {
    let options = cmd.data.options();

    let message = if options.is_empty() {
        let message = MessageBuilder::new()
            .push("Do you need a hug? Here you go, ")
            .mention(&cmd.user)
            .push(".")
            .build();

        message
    } else {
        if options.len() != 1 {
            error!("somehow, more options were passed to the command than should be possible. fix that.");
            return;
        }

        let ResolvedValue::User(target, _) = options[0].value else {
            error!("option passed to slash command is of incorrect type.");
            return;
        };

        let me: User = ctx.http.get_current_user().await.unwrap().into();

        if target == &me {
            MessageBuilder::new()
                .push("Aww, thank you ")
                .mention(&cmd.user)
                .push("!")
                .build()
        } else {
            MessageBuilder::new()
                .mention(target)
                .push(", you have been hugged by ")
                .mention(&cmd.user)
                .push("!")
                .build()
        }
    };

    // TODO api request
    let Ok(image) = get_image().await else {
        error!("couldnt get image");
        return;
    };
    let embed = CreateEmbed::new().image(image);

    let response_message = CreateInteractionResponseMessage::new()
        .content(message)
        .embed(embed);

    let response = CreateInteractionResponse::Message(response_message);

    if let Err(e) = cmd.create_response(&ctx.http, response).await {
        error!("error sending response message: {e}");
    }
}

pub fn register() -> CreateCommand {
    let target = CreateCommandOption::new(CommandOptionType::User, "target", "the user to hug");
    CreateCommand::new("hug")
        .description("hug a user (or yourself)")
        .add_option(target)
}
