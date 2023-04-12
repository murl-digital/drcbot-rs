pub mod admin;
pub mod controls;
pub mod playback;

use std::sync::Arc;
use tokio::sync::Mutex;

use poise::{
    send_application_reply,
    serenity_prelude::{ChannelId, GuildId, TypeMapKey},
};
use songbird::{Call, Songbird};

use crate::{
    commands::music::{
        admin::admin,
        controls::{now_playing, skip},
        playback::play,
    },
    Context, Error,
};

struct SkipVotes;

impl TypeMapKey for SkipVotes {
    type Value = Vec<u64>;
}

struct TrackRequester {
    name: String,
    avatar_url: String,
}

impl TypeMapKey for TrackRequester {
    type Value = TrackRequester;
}

#[poise::command(slash_command, subcommands("now_playing", "skip", "play", "admin"))]
pub async fn music(_: Context<'_>) -> Result<(), Error> {
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
