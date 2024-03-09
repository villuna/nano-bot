use serenity::builder::CreateCommand;

pub mod action;
pub mod help;
pub mod say_hi;

use help::HelpDetails;

#[derive(Clone, Debug)]
pub struct CommandDetails {
    pub command: CreateCommand,
    pub help: HelpDetails,
}
