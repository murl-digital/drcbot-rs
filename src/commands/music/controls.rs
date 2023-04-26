use chrono::Utc;

use poise::send_application_reply;

use crate::{
    commands::music::{get_client, SkipVotes, TrackRequester},
    local_get, Context, Error,
};

#[poise::command(slash_command, ephemeral, guild_only)]
pub async fn now_playing(ctx: Context<'_>) -> Result<(), Error> {
    let locale = ctx
        .locale()
        .expect("locale should always be available for slash commands");
    let guild = ctx.guild().unwrap();
    let channel = guild
        .voice_states
        .get(&ctx.author().id)
        .and_then(|v| v.channel_id);

    let current_channel = match channel {
        Some(channel) => channel,
        None => {
            send_application_reply(ctx, |r| {
                r.content(local_get(
                    &ctx.data.translator,
                    "commands_music_usernotinvc",
                    locale,
                ))
            })
            .await?;

            return Ok(());
        }
    };

    ctx.defer_ephemeral().await?;

    let manager = get_client(&ctx).await;

    let handler_lock = manager.get(ctx.guild_id().unwrap()).unwrap();

    let handler = handler_lock.lock().await;

    if handler
        .current_channel()
        .is_some_and(|c| c.0 == current_channel.0)
    {
        if let Some(current) = handler.queue().current() {
            let metadata = current.metadata();
            let type_map = current.typemap().read().await;
            let requester = type_map.get::<TrackRequester>();
            let color = match metadata.thumbnail.clone() {
                Some(t) => {
                    if let Ok(response) = reqwest::get(t).await {
                        if let Ok(image_bytes) = response.bytes().await {
                            if let Ok(image) = image::load_from_memory(&image_bytes) {
                                let pixels = image.to_rgb8();

                                if let Ok(mut pallette) = color_thief::get_palette(
                                    &pixels,
                                    color_thief::ColorFormat::Rgb,
                                    10,
                                    2,
                                ) {
                                    // sort by saturation
                                    pallette.sort_by(|a, b| saturation_from_rgb(a.r, a.g, a.b).partial_cmp(&saturation_from_rgb(b.r, b.g, b.b)).expect("NaN snuck in, something has gone wrong with pallette sorting"));
                                    pallette.reverse();
                                    Some(pallette[0])
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                None => None,
            };
            send_application_reply(ctx, |r| {
                r.embed(|e| {
                    e.title("Now Playing:")
                        .thumbnail(
                            metadata
                                .thumbnail
                                .clone()
                                .unwrap_or("https://http.cat/404".to_string()),
                        )
                        .description(format!(
                            "*{}*",
                            metadata
                                .source_url
                                .clone()
                                .unwrap_or("https://http.cat/404".to_string())
                        ))
                        .field(
                            "Title",
                            metadata.title.clone().unwrap_or("-".to_string()),
                            true,
                        )
                        .field(
                            "Artist",
                            metadata.artist.clone().unwrap_or("-".to_string()),
                            true,
                        )
                        .timestamp(Utc::now());

                    if let Some(color) = color {
                        e.color((color.r, color.g, color.b));
                    }

                    if let Some(requester) = requester {
                        e.footer(|f| {
                            f.icon_url(requester.avatar_url.to_owned())
                                .text(format!("Requested by {}", requester.name))
                        });
                    }

                    e
                })
            })
            .await?;
        }
    } else {
        send_application_reply(ctx, |r| {
            r.content(local_get(
                &ctx.data.translator,
                "commands_music_notwithbot",
                locale,
            ))
        })
        .await?;
    }

    Ok(())
}

#[poise::command(slash_command, ephemeral, guild_only)]
pub async fn skip(ctx: Context<'_>) -> Result<(), Error> {
    let locale = ctx
        .locale()
        .expect("locale should always be available for slash commands");
    let guild = ctx.guild().unwrap();
    let channel = guild
        .voice_states
        .get(&ctx.author().id)
        .and_then(|v| v.channel_id);

    let current_channel = match channel {
        Some(channel) => channel,
        None => {
            send_application_reply(ctx, |r| {
                r.content(local_get(
                    &ctx.data.translator,
                    "commands_music_usernotinvc",
                    locale,
                ))
            })
            .await?;

            return Ok(());
        }
    };

    ctx.defer_ephemeral().await?;

    let manager = get_client(&ctx).await;

    let handler_lock = manager.get(ctx.guild_id().unwrap()).unwrap();

    let handler = handler_lock.lock().await;

    if let Some(bot_channel) = handler.current_channel() {
        if bot_channel.0 == current_channel.0 {
            if let Some(current_track) = handler.queue().current() {
                let users_in_channel = guild
                    .voice_states
                    .iter()
                    .filter(|e| e.1.channel_id.is_some_and(|c| c.0 == bot_channel.0))
                    .count();

                let mut typemap = current_track.typemap().write().await;
                let votes = match typemap.get_mut::<SkipVotes>() {
                    Some(v) => v,
                    None => {
                        typemap.insert::<SkipVotes>(vec![]);

                        typemap
                            .get_mut::<SkipVotes>()
                            .expect("skip votes was literally just inserted")
                    }
                };

                if !votes.contains(&ctx.author().id.0) {
                    votes.push(ctx.author().id.0);
                    if votes.len() == (users_in_channel / 2) {
                        let _ = handler.queue().skip();
                    }
                    send_application_reply(ctx, |r| {
                        r.content(local_get(
                            &ctx.data.translator,
                            "commands_music_controls_skip_success",
                            locale,
                        ))
                    })
                    .await?;
                }
            } else {
                send_application_reply(ctx, |r| {
                    r.content(local_get(
                        &ctx.data.translator,
                        "commands_music_controls_skip_notplaying",
                        locale,
                    ))
                })
                .await?;
            }
        } else {
            send_application_reply(ctx, |r| {
                r.content(local_get(
                    &ctx.data.translator,
                    "commands_music_notwithbot",
                    locale,
                ))
            })
            .await?;
        }
    } else {
        send_application_reply(ctx, |r| {
            r.content(local_get(
                &ctx.data.translator,
                "commands_music_botnotinvc",
                locale,
            ))
        })
        .await?;
    }

    Ok(())
}

// dervied from https://donatbalipapp.medium.com/colours-maths-90346fb5abda
fn saturation_from_rgb(r: u8, g: u8, b: u8) -> f64 {
    let max_rgb = r.max(g).max(b) as f64;
    let min_rgb = r.min(g).min(b) as f64;
    let luminosity = 0.5 * (max_rgb + min_rgb);

    if luminosity < 1. {
        (max_rgb - min_rgb) / 1. - (2. * luminosity - 1.)
    } else {
        0.
    }
}
