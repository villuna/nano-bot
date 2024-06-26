use rand::seq::SliceRandom;
use serde::Deserialize;
use serenity::{
    all::{CommandInteraction, CommandOptionType, ResolvedValue, User},
    builder::{
        CreateCommand, CreateCommandOption, CreateEmbed, CreateInteractionResponse,
        CreateInteractionResponseMessage,
    },
    model::Colour,
    prelude::Context,
    utils::MessageBuilder,
};
use tracing::{error, info, instrument};

use crate::event_handler::Handler;

use super::{create_command_fn, help::HelpDetails, CommandDetails};

const EMBED_COLOURS: &[Colour] = &[
    Colour::FABLED_PINK,
    Colour::FOOYOO,
    Colour::ROSEWATER,
    Colour::BLURPLE,
    Colour::MEIBE_PINK,
    Colour::BLITZ_BLUE,
    Colour::FADED_PURPLE,
];

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ActionCommandData {
    kind: String,
    description: String,
    #[serde(rename = "targeted messages")]
    targeted_messages: Option<Vec<String>>,
    #[serde(rename = "lonely messages")]
    lonely_messages: Vec<String>,
    #[serde(rename = "nano messages")]
    nano_messages: Option<Vec<String>>,
}

impl ActionCommandData {
    fn targetable(&self) -> bool {
        self.targeted_messages.is_some() && self.nano_messages.is_some()
    }
}

#[derive(Clone, Debug, Deserialize)]
struct ImageResponse {
    url: String,
}

#[instrument(skip(client))]
async fn get_image(client: &reqwest::Client, kind: &str) -> reqwest::Result<String> {
    info!("sending request to otakugifs");
    let response = client.get(format!("https://api.otakugifs.xyz/gif?reaction={kind}"))
        .send()
        .await?
        .json::<ImageResponse>()
        .await?;

    info!("recieved reponse - okay");
    Ok(response.url)
}

pub async fn run(
    kind: &str,
    ctx: Context,
    cmd: &CommandInteraction,
    handler: Handler,
) {
    let options = cmd.data.options();
    let data = handler.actions.iter().find(|data| data.kind == kind).unwrap();
    let user_mention = MessageBuilder::new().mention(&cmd.user).build();

    let message = if options.is_empty()
        || matches!(options[0].value, ResolvedValue::User(u, _) if u == &cmd.user)
    {
        let mut rng = rand::thread_rng();
        let template = data.lonely_messages.choose(&mut rng).unwrap();
        template.replace("<user>", &user_mention)
    } else if data.targetable() {
        if options.len() != 1 {
            error!("somehow, more options were passed to the command than should be possible. fix that.");
            return;
        }

        let nano: User = ctx.http.get_current_user().await.unwrap().into();

        let ResolvedValue::User(target, _) = options[0].value else {
            error!("option passed to slash command is of incorrect type.");
            return;
        };

        let mut rng = rand::thread_rng();
        let target_mention = MessageBuilder::new().mention(target).build();

        let template = if target == &nano {
            data.nano_messages
                .as_ref()
                .unwrap()
                .choose(&mut rng)
                .unwrap()
        } else {
            data.targeted_messages
                .as_ref()
                .unwrap()
                .choose(&mut rng)
                .unwrap()
        };

        template
            .replace("<user>", &user_mention)
            .replace("<target>", &target_mention)
    } else {
        error!("command is not targetable, yet target was passed anyway");
        return;
    };

    let image = match get_image(&handler.http_client, kind).await {
        Ok(res) => res,
        Err(e) => {
            error!("couldnt get image: {e}");
            return;
        }
    };

    let colour = {
        let mut rng = rand::thread_rng();
        *EMBED_COLOURS.choose(&mut rng).unwrap()
    };

    let embed = CreateEmbed::new().image(image).colour(colour);
    let response_message = CreateInteractionResponseMessage::new()
        .content(message)
        .embed(embed);

    let response = CreateInteractionResponse::Message(response_message);

    if let Err(e) = cmd.create_response(&ctx.http, response).await {
        error!("error sending response message: {e}");
    }
}

pub fn register(commands_data: &[ActionCommandData]) -> Vec<CommandDetails> {
    let mut commands = Vec::new();

    for data in commands_data {
        let mut registration = CreateCommand::new(&data.kind).description(&data.description);
        let help = HelpDetails {
            name: data.kind.clone(),
            details: data.description.clone(),
            sub_commands: Vec::new(),
        };

        if data.targetable() {
            let target =
                CreateCommandOption::new(CommandOptionType::User, "target", "the user to target");
            registration = registration.add_option(target);
        }

        // Hacky solution but not so bad. Will try to use less memory later
        let kind: &'static str = data.kind.clone().leak();
        let command = create_command_fn(move |ctx, handler, cmd| async move {
            run(kind, ctx, &cmd, handler).await
        });

        commands.push(CommandDetails {
            name: data.kind.clone(),
            registration,
            help,
            command,
        });
    }

    commands
}
