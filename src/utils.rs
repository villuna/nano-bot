use tokio::sync::RwLock;
use tokio::time::Instant;

use serenity::{
    all::{GuildId, User},
    prelude::Context,
};

/// Gets the name of a user.
///
/// It will try to get a user's:
///
/// 1. Guild Nickname
/// 2. Global Nickname
/// 3. Username
///
/// In that order.
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

pub async fn get_nano_icon(ctx: &Context) -> String {
    let user = ctx.http.get_current_user().await.unwrap();

    user.avatar_url()
        .unwrap_or_else(|| user.default_avatar_url())
}

/// A thread safe, interior-mutable stopwatch
/// simply records instants in time and returns them
#[derive(Debug)]
pub struct SharedStopwatch(RwLock<Option<Instant>>);

impl SharedStopwatch {
    /// Creates a new, unset stopwatch
    pub fn new() -> Self {
        Self(RwLock::new(None))
    }

    /// Sets the stopwatch's start time to now
    pub async fn set_now(&self) {
        *self.0.write().await = Some(Instant::now());
    }

    /// Unsets the start time (sets it to None)
    pub async fn unset(&self) {
        *self.0.write().await = None;
    }

    /// Get the start time, if any
    pub async fn get(&self) -> Option<Instant> {
        *self.0.read().await
    }
}
