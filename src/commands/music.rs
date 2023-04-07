use std::sync::Arc;
use tokio::sync::Mutex;

use poise::{
    send_application_reply,
    serenity_prelude::{Attachment, ChannelId, GuildId},
};
use songbird::{input::Restartable, Call, Songbird};
use url::Url;

use crate::{Context, Error};

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
    if handler
        .current_channel()
        .is_some_and(|c| c.0 == current_channel.0)
    {
        let _ = handler.queue().skip();
    } else {
        send_application_reply(ctx, |r| r.content("not in a vc with the bot")).await?;
    }

    Ok(())
}

#[poise::command(slash_command, ephemeral, guild_only)]
pub async fn play_url(ctx: Context<'_>, url: Url) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    _play_url(ctx, url).await
}

#[poise::command(slash_command, ephemeral, guild_only)]
pub async fn play_attachment(ctx: Context<'_>, file: Attachment) -> Result<(), Error> {
    if let Some(content_type) = file.content_type {
        if content_type == "audio" {
            _play_url(
                ctx,
                Url::parse(&file.url).expect("this should be a valid url from discord"),
            )
            .await?
        } else {
            send_application_reply(ctx, |r| {
                r.content("this isn't an audio file, make sure it is")
            })
            .await?;
        }
    }

    send_application_reply(ctx, |r| r.content("no content type found")).await?;
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

    let source = match Restartable::ytdl(url, true).await {
        Ok(source) => source,
        Err(why) => {
            println!("problem starting source: {:?}", why);

            send_application_reply(ctx, |r| r.content("Error sourcing ffmpeg")).await?;

            return Ok(());
        }
    };

    handler.enqueue_source(source.into());

    send_application_reply(ctx, |r| r.content("your track has been queued now")).await?;

    Ok(())
}

/// gets the songbird client
/// this WILL PANIC if the client doesn't exist on the bot. is that a bad idea? maybe, i don't care
async fn get_client(ctx: &Context<'_>) -> Arc<Songbird> {
    match songbird::get(ctx.serenity_context).await {
        Some(client) => client,
        None => {
            println!("no songbird client exists, i will now cease to exist");
            // this is ignored because the bots about to crash, so this doesnt matter
            let _ = send_application_reply(*ctx, |r| r.content("Something has gone wrong internally. In fact, it's so bad that I'm going to crash after I send this message. Oops!")).await;
            panic!();
        }
    }
}

async fn get_handler(
    ctx: &Context<'_>,
    guild_id: &GuildId,
    connect_to: &ChannelId,
) -> Result<Arc<Mutex<Call>>, crate::Error> {
    let manager = get_client(ctx).await;

    let handler_lock = match manager.get(*guild_id) {
        Some(lock) => lock,
        None => {
            let (handler, result) = manager.join(*guild_id, *connect_to).await;

            if result.is_ok() {
                handler
            } else {
                return Err(result.unwrap_err().into());
            }
        }
    };

    Ok(handler_lock)
}
