use rand::seq::SliceRandom;
use serde::Deserialize;
use serenity::{
    all::{CommandInteraction, CommandOptionType, ResolvedOption, ResolvedValue, User},
    builder::{
        CreateCommand, CreateCommandOption, CreateEmbed, CreateInteractionResponse,
        CreateInteractionResponseMessage,
    },
    model::Colour,
    prelude::Context,
    utils::MessageBuilder,
};
use tracing::error;

const EMBED_COLOURS: &[Colour] = &[
    Colour::FABLED_PINK,
    Colour::FOOYOO,
    Colour::ROSEWATER,
    Colour::BLURPLE,
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

async fn get_image(kind: &str) -> reqwest::Result<String> {
    let response = reqwest::get(format!("https://api.otakugifs.xyz/gif?reaction={kind}"))
        .await?
        .json::<ImageResponse>()
        .await?;

    Ok(response.url)
}

pub async fn run(ctx: Context, cmd: &CommandInteraction, actions: &[ActionCommandData]) {
    let options = cmd.data.options();
    let ResolvedOption {
        name: kind,
        value: ResolvedValue::SubCommand(options),
        ..
    } = options[0].clone()
    else {
        error!("only option should be a sub command!");
        return;
    };

    let data = actions.iter().find(|data| data.kind == kind).unwrap();
    let user_mention = MessageBuilder::new().mention(&cmd.user).build();

    let message = if options.is_empty() {
        let mut rng = rand::thread_rng();
        let template = data.lonely_messages.choose(&mut rng).unwrap();
        template.replace("<user>", &user_mention)
    } else if data.targetable() {
        if options.len() != 1 {
            error!("somehow, more options were passed to the command than should be possible. fix that.");
            return;
        }

        let me: User = ctx.http.get_current_user().await.unwrap().into();

        let ResolvedValue::User(target, _) = options[0].value else {
            error!("option passed to slash command is of incorrect type.");
            return;
        };

        let mut rng = rand::thread_rng();
        let target_mention = MessageBuilder::new().mention(&cmd.user).build();

        let template = if target == &me {
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

    let image = match get_image(kind).await {
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

pub fn register(commands: &[ActionCommandData]) -> CreateCommand {
    let mut command =
        CreateCommand::new("action").description("perform an action, as represented by a gif");

    for data in commands {
        let mut action =
            CreateCommandOption::new(CommandOptionType::SubCommand, &data.kind, &data.description);

        if data.targetable() {
            let target =
                CreateCommandOption::new(CommandOptionType::User, "target", "the user to target");
            action = action.add_sub_option(target);
        }

        command = command.add_option(action);
    }

    command
}
