use poise::{
    send_application_reply,
    serenity_prelude::{Attachment, Channel},
};
use songbird::input::Restartable;
use url::Url;

use crate::{local_get, Context, Error, MIME_AUDIO_REGEX};

use super::{
    get_color_from_thumbnail, get_handler, make_now_playing_embed, QuickLeave, TrackRequester,
};

#[poise::command(slash_command, subcommands("url", "attachment"))]
#[allow(clippy::unused_async)]
pub async fn play(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, ephemeral, guild_only)]
async fn url(ctx: Context<'_>, url: Url, quick_leave: Option<bool>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    _play_url(ctx, url, quick_leave).await
}

#[poise::command(slash_command, ephemeral, guild_only)]
async fn attachment(
    ctx: Context<'_>,
    file: Attachment,
    quick_leave: Option<bool>,
) -> Result<(), Error> {
    let locale = ctx
        .locale()
        .expect("locales should always be available for slash commands");
    if let Some(content_type) = file.content_type {
        if MIME_AUDIO_REGEX.is_match(&content_type) {
            ctx.defer_ephemeral().await?;
            _play_url(
                ctx,
                Url::parse(&file.url).expect("this should be a valid url from discord"),
                quick_leave,
            )
            .await?;
        } else {
            send_application_reply(ctx, |r| {
                r.content(local_get(
                    &ctx.data.translator,
                    "commands_music_playback_attachment_notaudio",
                    locale,
                ))
            })
            .await?;
        }
    } else {
        send_application_reply(ctx, |r| {
            r.content(local_get(
                &ctx.data.translator,
                "commands_music_playback_attachment_nocontenttype",
                locale,
            ))
        })
        .await?;
    }

    Ok(())
}

async fn _play_url(ctx: Context<'_>, url: Url, quick_leave: Option<bool>) -> Result<(), Error> {
    let locale = ctx
        .locale()
        .expect("locales should always be available for slash commands");
    let guild = ctx.guild().expect("this is supposed to be guild only");
    let guild_id = guild.id;

    let channel = guild
        .voice_states
        .get(&ctx.author().id)
        .and_then(|v| v.channel_id);

    let Some(connect_to) = channel else {
        send_application_reply(ctx, |r| {
            r.content(local_get(
                &ctx.data.translator,
                "commands_music_usernotinvc",
                locale,
            ))
        })
        .await?;

        return Ok(());
    };

    let handler_lock = get_handler(&ctx, &guild_id, &connect_to).await?;

    let mut handler = handler_lock.lock().await;

    if let Some(current_channel) = handler.current_channel() {
        if current_channel.0 != connect_to.0 {
            send_application_reply(ctx, |r| {
                r.content(local_get(
                    &ctx.data.translator,
                    "commands_music_alreadyinvc",
                    locale,
                ))
            })
            .await?;

            return Ok(());
        }
    };

    let source = match Restartable::ytdl(url, false).await {
        Ok(source) => source,
        Err(why) => {
            println!("problem starting source: {:?}", why);

            send_application_reply(ctx, |r| {
                r.content(local_get(
                    &ctx.data.translator,
                    "commands_music_playback_ffmpeg",
                    locale,
                ))
            })
            .await?;

            return Ok(());
        }
    };

    let handle = handler.enqueue_source(source.into());

    let (name, avatar_url) = (ctx.author_member().await).map_or_else(
        || (ctx.author().name.clone(), ctx.author().face()),
        |member| (member.display_name().into_owned(), member.face()),
    );

    let mut type_map = handle.typemap().write().await;
    type_map.insert::<TrackRequester>(TrackRequester { name, avatar_url });
    if quick_leave.is_some_and(|q| q) {
        type_map.insert::<QuickLeave>(QuickLeave);
    }
    drop(type_map);

    send_application_reply(ctx, |r| {
        r.content(local_get(
            &ctx.data.translator,
            "commands_music_playback_queued",
            locale,
        ))
    })
    .await?;

    if handler.queue().len() == 1 {
        let http = ctx.serenity_context.http.clone();
        if let Some(current_channel) = handler.current_channel() {
            if let Ok(Channel::Guild(current_channel)) = http.get_channel(current_channel.0).await {
                let color = get_color_from_thumbnail(handle.metadata()).await;

                let type_map = handle.typemap().read().await;
                if let Err(why) = current_channel
                    .send_message(http, |m| {
                        m.add_embed(|e| {
                            make_now_playing_embed(
                                e,
                                handle.metadata(),
                                color,
                                type_map.get::<TrackRequester>(),
                            )
                        })
                    })
                    .await
                {
                    println!("Error sending now playing message: {:?}", why);
                }
            }
        }
    }

    Ok(())
}
