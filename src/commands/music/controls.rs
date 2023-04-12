use chrono::Utc;

use poise::send_application_reply;

use crate::{
    commands::music::{get_client, SkipVotes, TrackRequester},
    Context, Error,
};

#[poise::command(slash_command, ephemeral, guild_only)]
pub async fn now_playing(ctx: Context<'_>) -> Result<(), Error> {
    let guild = ctx.guild().unwrap();
    let channel = guild
        .voice_states
        .get(&ctx.author().id)
        .and_then(|v| v.channel_id);

    let current_channel = match channel {
        Some(channel) => channel,
        None => {
            send_application_reply(ctx, |r| r.content("you're not in a vc lmao")).await?;

            return Ok(());
        }
    };

    ctx.defer_ephemeral().await?;

    let manager = get_client(&ctx).await;

    let handler_lock = manager.get(ctx.guild_id().unwrap()).unwrap();

    let handler = handler_lock.lock().await;

    // is_some_and should be stabilized soon
    if handler
        .current_channel()
        .is_some_and(|c| c.0 == current_channel.0)
    {
        if let Some(current) = handler.queue().current() {
            let metadata = current.metadata();
            let type_map = current.typemap().read().await;
            let requester = type_map.get::<TrackRequester>();
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
        send_application_reply(ctx, |r| r.content("not in a vc with the bot")).await?;
    }

    Ok(())
}

#[poise::command(slash_command, ephemeral, guild_only)]
pub async fn skip(ctx: Context<'_>) -> Result<(), Error> {
    let guild = ctx.guild().unwrap();
    let channel = guild
        .voice_states
        .get(&ctx.author().id)
        .and_then(|v| v.channel_id);

    let current_channel = match channel {
        Some(channel) => channel,
        None => {
            send_application_reply(ctx, |r| r.content("you're not in a vc lmao")).await?;

            return Ok(());
        }
    };

    ctx.defer_ephemeral().await?;

    let manager = get_client(&ctx).await;

    let handler_lock = manager.get(ctx.guild_id().unwrap()).unwrap();

    let handler = handler_lock.lock().await;

    // is_some_and should be stabilized soon
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
                    send_application_reply(ctx, |r| r.content("your vote has been counted"))
                        .await?;
                }
            } else {
                send_application_reply(ctx, |r| r.content("no tracks are playing right now"))
                    .await?;
            }
        } else {
            send_application_reply(ctx, |r| r.content("you're not in a vc with the bot")).await?;
        }
    } else {
        send_application_reply(ctx, |r| r.content("not in any vcs atm")).await?;
    }

    Ok(())
}
