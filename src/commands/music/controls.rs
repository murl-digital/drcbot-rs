use poise::send_application_reply;

use crate::{
    commands::music::{
        get_client, get_color_from_thumbnail, make_now_playing_embed, SkipVotes, TrackRequester,
    },
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
            let color = get_color_from_thumbnail(metadata).await;
            send_application_reply(ctx, |r| {
                r.embed(|e| make_now_playing_embed(e, metadata, color, requester))
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
