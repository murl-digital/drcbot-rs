use std::{cell::LazyCell, sync::LazyLock};

use poise::{
    send_application_reply,
    serenity_prelude::{Attachment, Channel, CreateMessage}, CreateReply,
};
use reqwest::Client;
use songbird::input::{Compose, YoutubeDl};
use url::Url;

use crate::{commands::music::TrackMetadata, local_get, Context, Error, MIME_AUDIO_REGEX};

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
            send_application_reply(ctx, CreateReply::default().content(local_get(
                    &ctx.data.translator,
                    "commands_music_playback_attachment_notaudio",
                    locale,
                ))
            )
            .await?;
        }
    } else {
        send_application_reply(ctx, CreateReply::default().content(local_get(
                &ctx.data.translator,
                "commands_music_playback_attachment_nocontenttype",
                locale,
            ))
        )
        .await?;
    }

    Ok(())
}

async fn _play_url(ctx: Context<'_>, url: Url, quick_leave: Option<bool>) -> Result<(), Error> {
    static CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

    let locale = ctx
        .locale()
        .expect("locales should always be available for slash commands");
    let guild = ctx.guild().expect("this is supposed to be guild only").clone();
    let guild_id = guild.id;

    let channel = guild
        .voice_states
        .get(&ctx.author().id)
        .and_then(|v| v.channel_id);

    let Some(connect_to) = channel else {
        send_application_reply(ctx, CreateReply::default().content(local_get(
                &ctx.data.translator,
                "commands_music_usernotinvc",
                locale,
            ))
        )
        .await?;

        return Ok(());
    };

    let handler_lock = get_handler(&ctx, &guild_id, &connect_to).await?;

    let mut handler = handler_lock.lock().await;

    if let Some(current_channel) = handler.current_channel() {
        if current_channel != connect_to.into() {
            send_application_reply(ctx, CreateReply::default().content(local_get(
                    &ctx.data.translator,
                    "commands_music_alreadyinvc",
                    locale,
                ))
            )
            .await?;

            return Ok(());
        }
    };

    let mut source = YoutubeDl::new(CLIENT.clone(), url.into());

    //  {
    //     Ok(source) => source,
    //     Err(why) => {
    //         println!("problem starting source: {:?}", why);

    //         send_application_reply(ctx, |r| {
    //             r.content(local_get(
    //                 &ctx.data.translator,
    //                 "commands_music_playback_ffmpeg",
    //                 locale,
    //             ))
    //         })
    //         .await?;

    //         return Ok(());
    //     }
    // }

    let metadata = source.aux_metadata().await?;

    let handle = handler.enqueue(source.into()).await;

    let (name, avatar_url) = (ctx.author_member().await).map_or_else(
        || (ctx.author().name.clone(), ctx.author().face()),
        |member| (member.display_name().to_owned(), member.face()),
    );

    let mut type_map = handle.typemap().write().await;
    type_map.insert::<TrackRequester>(TrackRequester { name, avatar_url });

    type_map.insert::<TrackMetadata>(metadata);
    if quick_leave.is_some_and(|q| q) {
        type_map.insert::<QuickLeave>(QuickLeave);
    }

    drop(type_map);

    send_application_reply(ctx, CreateReply::default().content(local_get(
            &ctx.data.translator,
            "commands_music_playback_queued",
            locale,
        ))
    )
    .await?;

    if handler.queue().len() == 1 {
        let http = ctx.serenity_context.http.clone();
        if let Some(current_channel) = handler.current_channel() {
            if let Ok(Channel::Guild(current_channel)) = http.get_channel(current_channel.0.into()).await {
                let type_map = handle.typemap().read().await;
                let metadata = type_map.get::<TrackMetadata>().expect("metadata MUST be available at this point");


                let color = get_color_from_thumbnail(metadata).await;

                if let Err(why) = current_channel
                    .send_message(http, CreateMessage::new().add_embed(make_now_playing_embed(
                                metadata,
                                color,
                                type_map.get::<TrackRequester>(),
                            )
                        )
                    )
                    .await
                {
                    tracing::warn!("Error sending now playing message: {:?}", why);
                }
            }
        }
    }

    Ok(())
}
