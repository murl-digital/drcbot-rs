#![feature(is_some_and)]

use commands::music::{now_playing, play_attachment, play_url, skip};
use lazy_static::lazy_static;
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

use commands::reactionroles::*;
use regex::Regex;
use songbird::SerenityInit;

#[derive(Debug)]
pub struct Data {
    client: Arc<Client>,
}

lazy_static! {
    pub static ref ID_REGEX: Regex = Regex::new(r"rr:(\d{18})").unwrap();
    pub static ref MIME_AUDIO_REGEX: Regex = Regex::new(r"audio/.+").unwrap();
}

#[derive(Deserialize)]
struct Config {
    token: String,
    mongodb_url: String,
}

#[tokio::main]
async fn main() {
    let config: Config = toml::from_str(
        &tokio::fs::read_to_string("config.toml")
            .await
            .expect("config doesn't exist"),
    )
    .expect("invalid config");

    let mongo_client = Client::with_uri_str(config.mongodb_url)
        .await
        .expect("problem connecting to mongodb");

    let framework = Framework::builder()
        .token(config.token)
        .intents(GatewayIntents::non_privileged() | GatewayIntents::GUILD_VOICE_STATES)
        .client_settings(|c| {
            c.register_songbird()
        })
        .options(FrameworkOptions {
            commands: vec![
                add_role(),
                init(),
                add_message(),
                play_url(),
                play_attachment(),
                skip(),
                now_playing(),
            ],
            event_handler: |ctx, event, _framework, _data| {
                Box::pin(async move {
                    if let Event::InteractionCreate { interaction } = event {
                        handle_reaction_roles(ctx, interaction).await?
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
                    client: Arc::new(mongo_client),
                })
            })
        });

    framework.run().await.unwrap();
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
