use poise::{send_application_reply, serenity_prelude::Attachment};
use songbird::input::Restartable;
use url::Url;

use crate::{Context, Error, MIME_AUDIO_REGEX};

use super::{get_handler, TrackRequester};

#[poise::command(slash_command, subcommands("url", "attachment"))]
pub async fn play(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, ephemeral, guild_only)]
async fn url(ctx: Context<'_>, url: Url) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    _play_url(ctx, url).await
}

#[poise::command(slash_command, ephemeral, guild_only)]
async fn attachment(ctx: Context<'_>, file: Attachment) -> Result<(), Error> {
    if let Some(content_type) = file.content_type {
        if MIME_AUDIO_REGEX.is_match(&content_type) {
            ctx.defer_ephemeral().await?;
            _play_url(
                ctx,
                Url::parse(&file.url).expect("this should be a valid url from discord"),
            )
            .await?;
        } else {
            send_application_reply(ctx, |r| {
                r.content("this isn't an audio file, make sure it is")
            })
            .await?;
        }
    } else {
        send_application_reply(ctx, |r| r.content("no content type found")).await?;
    }

    Ok(())
}

async fn _play_url(ctx: Context<'_>, url: Url) -> Result<(), Error> {
    let guild = ctx.guild().unwrap();
    let guild_id = guild.id;

    let channel = guild
        .voice_states
        .get(&ctx.author().id)
        .and_then(|v| v.channel_id);

    let connect_to = match channel {
        Some(channel) => channel,
        None => {
            send_application_reply(ctx, |r| r.content("you're not in a vc lmao")).await?;

            return Ok(());
        }
    };

    let handler_lock = get_handler(&ctx, &guild_id, &connect_to).await?;

    let mut handler = handler_lock.lock().await;

    if let Some(current_channel) = handler.current_channel() {
        if current_channel.0 != connect_to.0 {
            send_application_reply(ctx, |r| r.content("I'm already in a call")).await?;

            return Ok(());
        }
    };

    let source = match Restartable::ytdl(url, false).await {
        Ok(source) => source,
        Err(why) => {
            println!("problem starting source: {:?}", why);

            send_application_reply(ctx, |r| r.content("Error sourcing ffmpeg")).await?;

            return Ok(());
        }
    };

    let handle = handler.enqueue_source(source.into());

    let (name, avatar_url) = match ctx.author_member().await {
        Some(member) => (member.display_name().into_owned(), member.face()),
        None => (ctx.author().name.clone(), ctx.author().face()),
    };

    let mut type_map = handle.typemap().write().await;
    type_map.insert::<TrackRequester>(TrackRequester { name, avatar_url });

    send_application_reply(ctx, |r| r.content("your track has been queued now")).await?;

    Ok(())
}
