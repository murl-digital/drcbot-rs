#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::unwrap_used)]

use commands::{music::music, reaction_roles::reaction_roles};
use data::Database;
use lazy_static::lazy_static;
use locale::Translator;
use mongodb::Client;
use poise::builtins::register_globally;
use poise::serenity_prelude::{GatewayIntents, Interaction, MessageBuilder};
use poise::{serenity_prelude as serenity, ApplicationContext, FrameworkError};
use poise::{Event, Framework, FrameworkOptions};
use serde::Deserialize;
use std::sync::Arc;

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

lazy_static! {
    pub static ref ID_REGEX: Regex = Regex::new(r"rr:(\d{18})").expect("ID_REGEX didn't compile");
    pub static ref MIME_AUDIO_REGEX: Regex =
        Regex::new(r"audio/.+").expect("MIME_AUDIO_REGEX didn't compile");
}

#[derive(Deserialize)]
struct Config {
    token: String,
    mongodb_url: String,
    mongodb_database: String,
}

#[tokio::main]
async fn main() -> Result<(), serenity::Error> {
    let config: Config = toml::from_str(
        &tokio::fs::read_to_string("config.toml")
            .await
            .expect("config doesn't exist"),
    )
    .expect("invalid config");

    let translator = Translator::new("locale.toml")
        .await
        .expect("translator required for working bot");

    let mongo_client = Client::with_uri_str(config.mongodb_url)
        .await
        .expect("problem connecting to mongodb");

    let framework = Framework::builder()
        .token(config.token)
        .intents(GatewayIntents::non_privileged() | GatewayIntents::GUILD_VOICE_STATES)
        .client_settings(songbird::SerenityInit::register_songbird)
        .options(FrameworkOptions {
            commands: vec![reaction_roles(), music()],
            event_handler: |ctx, event, _framework, _data| {
                Box::pin(async move {
                    if let Event::InteractionCreate { interaction } = event {
                        handle_reaction_roles(ctx, interaction).await?;
                    }
                    Ok(())
                })
            },
            on_error: |err| {
                Box::pin(async move {
                    if let FrameworkError::Command { error, ctx } = err {
                        println!("error running command: {:?}", error);
                        println!("context for debugging: {:?}", ctx);
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
        });

    framework.run().await
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
            .unwrap_or_else(|_| panic!("key {} doesn't exist", key))
    })
}

async fn handle_reaction_roles(
    ctx: &serenity::Context,
    interaction: &Interaction,
) -> Result<(), Error> {
    if let Some(mut component) = interaction.clone().message_component() {
        component.defer(&ctx.http).await?;

        if let Some(captures) = ID_REGEX.captures(&component.data.custom_id) {
            if let Ok(parsed) = captures[1].parse() {
                let role_id = serenity::RoleId(parsed);
                if let Some(ref mut member) = component.member {
                    if member.roles.iter().any(|r| r == &role_id) {
                        member.remove_role(&ctx, &role_id).await?;
                        component
                            .create_followup_message(&ctx, |r| {
                                r.ephemeral(true).content(
                                    MessageBuilder::new()
                                        .push("you no longer have the ")
                                        .role(role_id)
                                        .push(" role"),
                                )
                            })
                            .await?;
                    } else {
                        member.add_role(&ctx, &role_id).await?;
                        component
                            .create_followup_message(&ctx, |r| {
                                r.ephemeral(true).content(
                                    MessageBuilder::new()
                                        .push("you got the ")
                                        .role(role_id)
                                        .push(" role"),
                                )
                            })
                            .await?;
                    }
                }
            }
        }
    }

    Ok(())
}
