use serenity::{
    all::{GuildId, User},
    prelude::Context,
};

pub async fn get_name(ctx: &Context, user: &User, guild: Option<&GuildId>) -> String {
    let nickname = match guild {
        Some(id) => user.nick_in(&ctx.http, id).await,
        None => None,
    };

    nickname
        .as_ref()
        .or(user.global_name.as_ref())
        .unwrap_or(&user.name)
        .clone()
}
