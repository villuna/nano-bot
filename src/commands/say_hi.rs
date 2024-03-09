use crate::utils::get_name;
use serenity::{
    all::{CommandInteraction, CreateCommand},
    builder::{CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage},
    prelude::Context,
};
use tracing::error;

use super::{help::HelpDetails, CommandDetails};

pub async fn run(ctx: Context, cmd: &CommandInteraction) {
    let name = get_name(&ctx, &cmd.user, cmd.guild_id.as_ref()).await;

    let embed = CreateEmbed::new()
        .title(format!("Hi {}!!\nMy name is Shinonome Nano!", name))
        .image("https://media1.tenor.com/m/yan7w90ts3MAAAAC/nichijou.gif");

    let message = CreateInteractionResponseMessage::new().embed(embed);
    let builder = CreateInteractionResponse::Message(message);

    if let Err(e) = cmd.create_response(&ctx.http, builder).await {
        error!("error sending response to sayhi command: {e}");
    }
}

pub fn register() -> CommandDetails {
    let command = CreateCommand::new("sayhi").description("Say hi to Nano");
    let help = HelpDetails {
        name: "sayhi".to_string(),
        details: "Say hi to Nano".to_string(),
        ..Default::default()
    };

    CommandDetails { command, help }
}
