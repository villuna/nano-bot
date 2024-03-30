use std::pin::Pin;
use std::future::Future;

use serenity::{all::CommandInteraction, builder::CreateCommand, prelude::Context};

pub mod action;
pub mod help;
pub mod say_hi;

use help::HelpDetails;

use crate::event_handler::Handler;

// pwetty pleeeeeeeease stablise async closures :3333

/// An async function that performs a command response.
///
/// Takes in a [Context], [Handler] and
/// [CommandInteraction] and returns a future with no result.
///
/// This is a boxed trait object which returns boxed trait object. To create one from a regular
/// closure, use [create_command_fn]
pub type CommandFn = Box<
    dyn Fn(
            Context,
            Handler,
            CommandInteraction,
        ) -> Pin<Box<dyn Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

/// Takes in a function that returns a future, and turns it into a boxed function that returns a
/// boxed future. Useful so we can store a bunch of type-erased async closures together.
pub fn create_command_fn<F, R>(f: F) -> CommandFn
where
    F: Fn(Context, Handler, CommandInteraction) -> R + Send + Sync + 'static,
    R: Future<Output = ()> + Send + 'static,
{
    Box::new(move |ctx, handler, cmd| Box::pin(f(ctx, handler, cmd)))
}

pub struct CommandDetails {
    pub name: String,
    pub registration: CreateCommand,
    pub help: HelpDetails,
    pub command: CommandFn,
}
