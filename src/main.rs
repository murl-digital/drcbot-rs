#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::unwrap_used)]

use commands::{music::music, reaction_roles::reaction_roles};
use data::Database;
use locale::Translator;
use mongodb::Client;
use poise::builtins::register_globally;
use poise::serenity_prelude::{
    CreateInteractionResponseFollowup, FullEvent, GatewayIntents, Interaction, MessageBuilder,
    RoleId,
};
use poise::{serenity_prelude as serenity, ApplicationContext, FrameworkError};
use poise::{Framework, FrameworkOptions};
use serde::Deserialize;
use songbird::SerenityInit;
use thiserror::Error;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use std::sync::{Arc, LazyLock};

pub type Context<'a> = ApplicationContext<'a, Data, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

mod commands;
mod data;
mod locale;

use regex::Regex;

#[derive(Debug)]
pub struct Data {
    pub database: Arc<Database>,
    pub translator: Arc<Translator>,
}

pub static ID_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"rr:(\d{18})").expect("ID_REGEX didn't compile"));
pub static MIME_AUDIO_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"audio/.+").expect("MIME_AUDIO_REGEX didn't compile"));

#[derive(Deserialize)]
struct Config {
    token: String,
    mongodb_url: String,
    mongodb_database: String,
}

#[derive(Error, Debug)]
enum StartupError {
    #[error("can't connect to journald for logging: {0}")]
    JournaldConnection(std::io::Error),
    #[error("problem reading config file from disk: {0}")]
    ReadConfig(std::io::Error),
    #[error("problem parsing config: {0}")]
    ParseConfig(toml::de::Error),
    #[error("problem initializing translator: {0}")]
    InitTranslator(locale::InitError),
    #[error("problem connecting to database: {0}")]
    Database(mongodb::error::Error)
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::registry().with(tracing_journald::layer().map_err(StartupError::JournaldConnection)?).init();

    let config: Config = toml::from_str(
        &tokio::fs::read_to_string("config.toml").await.map_err(StartupError::ReadConfig)?
    ).map_err(StartupError::ParseConfig)?;

    let translator = Translator::new("locale.toml")
        .await
        .map_err(StartupError::InitTranslator)?;

    let mongo_client = Client::with_uri_str(config.mongodb_url)
        .await
        .map_err(StartupError::Database)?;

    let framework = Framework::builder()
        .options(FrameworkOptions {
            commands: vec![reaction_roles(), music()],
            event_handler: |ctx, event, _framework, _data| {
                Box::pin(async move {
                    if let FullEvent::InteractionCreate { interaction } = event {
                        handle_reaction_roles(ctx, interaction).await?;
                    }
                    Ok(())
                })
            },
            on_error: |err| {
                Box::pin(async move {
                    if let FrameworkError::Command { error, ctx, .. } = err {
                        tracing::error!(
                            "error running command: {error:?} \n context for debugging: {ctx:?}"
                        );
                    }
                })
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    database: Arc::new(Database::new(mongo_client, config.mongodb_database)),
                    translator: Arc::new(translator),
                })
            })
        })
        .build();

    let mut client = serenity::ClientBuilder::new(
        config.token,
        GatewayIntents::non_privileged() | GatewayIntents::GUILD_VOICE_STATES,
    )
    .register_songbird()
    .framework(framework)
    .await?;

    Ok(client.start().await?)
}

/// Gets a localized string based on a given key.
///
/// # Panics
///
/// Panics if the provided key doesn't exist, since it is assumed that the key is required for the bot to run.
#[must_use]
pub fn local_get(translator: &Translator, key: &str, locale: &str) -> String {
    translator.get(key, locale).unwrap_or_else(|_| {
        translator
            .get(key, "en-US")
            .unwrap_or_else(|_| panic!("key {key} doesn't exist"))
    })
}

async fn handle_reaction_roles(
    ctx: &serenity::Context,
    interaction: &Interaction,
) -> Result<(), Error> {
    if let Some(mut component) = interaction.clone().message_component() {
        component.defer(&ctx).await?;

        if let Some(captures) = ID_REGEX.captures(&component.data.custom_id) {
            if let Ok(role_id) = captures[1].parse::<RoleId>() {
                if let Some(ref mut member) = component.member {
                    if member.roles.iter().any(|r| r == &role_id) {
                        member.remove_role(&ctx, &role_id).await?;
                        component
                            .create_followup(
                                &ctx,
                                CreateInteractionResponseFollowup::new()
                                    .ephemeral(true)
                                    .content(
                                        MessageBuilder::new()
                                            .push("you no longer have the ")
                                            .role(role_id)
                                            .push(" role")
                                            .build(),
                                    ),
                            )
                            .await?;
                    } else {
                        member.add_role(&ctx, &role_id).await?;
                        component
                            .create_followup(
                                &ctx,
                                CreateInteractionResponseFollowup::new()
                                    .ephemeral(true)
                                    .content(
                                        MessageBuilder::new()
                                            .push("you got the ")
                                            .role(role_id)
                                            .push(" role")
                                            .build(),
                                    ),
                            )
                            .await?;
                    }
                }
            }
        }
    }

    Ok(())
}
