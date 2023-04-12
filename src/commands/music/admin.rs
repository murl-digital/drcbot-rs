use poise::send_application_reply;

use crate::{commands::music::get_client, Context, Error};

#[poise::command(
    slash_command,
    subcommands("force_skip"),
    required_permissions = "MANAGE_MESSAGES"
)]
pub async fn admin(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, ephemeral, guild_only)]
async fn force_skip(ctx: Context<'_>) -> Result<(), Error> {
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
