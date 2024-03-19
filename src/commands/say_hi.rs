use crate::utils::get_name;
use rand::{seq::SliceRandom, thread_rng};
use serde::Deserialize;
use serenity::{
    all::{CommandInteraction, CreateCommand},
    builder::{CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage},
    prelude::Context,
    utils::MessageBuilder,
};
use tracing::error;

use super::{help::HelpDetails, CommandDetails, create_command_fn};

#[derive(Deserialize, Debug, Clone)]
pub struct SayHiData {
    message: String,
    gif: String,
}

pub async fn run(ctx: Context, cmd: &CommandInteraction, data: &[SayHiData]) {
    let name = get_name(&ctx, &cmd.user, cmd.guild_id.as_ref()).await;
    let sanitised_name = MessageBuilder::new().push_safe(name).build();

    let message = data.choose(&mut thread_rng()).unwrap();
    let title = &message.message;
    let gif = &message.gif;

    let embed = CreateEmbed::new()
        .title(title.replace("<name>", &sanitised_name))
        .image(gif);

    let message = CreateInteractionResponseMessage::new().embed(embed);
    let builder = CreateInteractionResponse::Message(message);

    if let Err(e) = cmd.create_response(&ctx.http, builder).await {
        error!("error sending response to sayhi command: {e}");
    }
}

pub fn register() -> CommandDetails {
    let registration = CreateCommand::new("sayhi").description("Say hi to Nano");
    let help = HelpDetails {
        name: "sayhi".to_string(),
        details: "Say hi to Nano".to_string(),
        ..Default::default()
    };

    let command = create_command_fn(|ctx, handler, cmd| async move {
        run(ctx, &cmd, &handler.say_hi_data).await
    });

    CommandDetails {
        name: "sayhi".to_owned(),
        registration,
        help,
        command,
    }
}
