use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Instant;

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

#[derive(Debug)]
pub struct SharedStopwatch(Arc<RwLock<Option<Instant>>>);

impl SharedStopwatch {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(None)))
    }

    pub async fn set_now(&self) {
        *self.0.write().await = Some(Instant::now());
    }

    pub async fn reset(&self) {
        *self.0.write().await = None;
    }

    pub async fn get(&self) -> Option<Instant> {
        *self.0.read().await
    }
}
