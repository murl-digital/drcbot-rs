use poise::send_application_reply;

use crate::{commands::music::get_client, local_get, Context, Error};

#[poise::command(
    slash_command,
    subcommands("force_skip", "stop"),
    required_permissions = "MANAGE_MESSAGES"
)]
#[allow(clippy::unused_async)]
pub async fn admin(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, ephemeral, guild_only)]
async fn force_skip(ctx: Context<'_>) -> Result<(), Error> {
    let locale = ctx
        .locale()
        .expect("locale should always be available for slash commands");
    let guild = ctx.guild().expect("no guild provided for a guild only command");
    let channel = guild
        .voice_states
        .get(&ctx.author().id)
        .and_then(|v| v.channel_id);

    let Some(current_channel) = channel else {
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

    ctx.defer_ephemeral().await?;

    let manager = get_client(&ctx).await;

    let handler_lock = manager.get(guild.id).unwrap();

    let handler = handler_lock.lock().await;

    if handler
        .current_channel()
        .is_some_and(|c| c.0 == current_channel.0)
    {
        let _ = handler.queue().skip();
        drop(handler);
        send_application_reply(ctx, |r| {
            r.content(local_get(
                &ctx.data.translator,
                "commands_music_admin_forceskip_success",
                locale,
            ))
        })
        .await?;
    } else {
        drop(handler);
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
async fn stop(ctx: Context<'_>, leave: Option<bool>) -> Result<(), Error> {
    let locale = ctx
        .locale()
        .expect("locale should always be available for slash commands");
    let guild = ctx.guild().expect("no guild for guild only command");
    let channel = guild
        .voice_states
        .get(&ctx.author().id)
        .and_then(|v| v.channel_id);

    let Some(current_channel) = channel else {
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

    ctx.defer_ephemeral().await?;

    let manager = get_client(&ctx).await;

    let handler_lock = manager.get(guild.id).unwrap();

    let mut handler = handler_lock.lock().await;

    if handler
        .current_channel()
        .is_some_and(|c| c.0 == current_channel.0)
    {
        handler.queue().stop();
        if leave.unwrap_or(false) {
            handler.leave().await?;
        }
        drop(handler);
        send_application_reply(ctx, |r| {
            r.content(local_get(
                &ctx.data.translator,
                "commands_music_admin_stop_success",
                locale,
            ))
        })
        .await?;
    } else {
        drop(handler);
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
