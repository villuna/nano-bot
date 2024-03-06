use serenity::{
    all::{CommandInteraction, CreateCommand},
    builder::{CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage},
    prelude::Context,
};
use tracing::error;

pub async fn run(ctx: Context, cmd: &CommandInteraction) {
    let nickname = match cmd.guild_id {
        Some(id) => cmd.user.nick_in(&ctx.http, id).await,
        None => None,
    };

    let name = nickname.as_ref()
        .or(cmd.user.global_name.as_ref())
        .unwrap_or(&cmd.user.name);

    let embed = CreateEmbed::new()
        .title(format!("Hi {}!!\nMy name is Shinonome Nano!", name))
        .image("https://media1.tenor.com/m/yan7w90ts3MAAAAC/nichijou.gif");

    let message = CreateInteractionResponseMessage::new().embed(embed);
    let builder = CreateInteractionResponse::Message(message);

    if let Err(e) = cmd.create_response(&ctx.http, builder).await {
        error!("error sending response to sayhi command: {e}");
    }
}

pub fn register() -> CreateCommand {
    CreateCommand::new("sayhi").description("say hi to nano")
}
