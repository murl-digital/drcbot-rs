pub mod admin;
pub mod controls;
pub mod playback;

use chrono::Utc;
use rgb::RGB;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

use crate::serenity::async_trait;
use poise::{
    send_application_reply,
    serenity_prelude::{Channel, ChannelId, CreateEmbed, GuildId, Http, TypeMapKey},
};
use songbird::{input::Metadata, Call, Event, EventContext, EventHandler, Songbird};

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

#[derive(Clone)]
struct TrackRequester {
    name: String,
    avatar_url: String,
}

impl TypeMapKey for TrackRequester {
    type Value = Self;
}

#[poise::command(slash_command, subcommands("now_playing", "skip", "play", "admin"))]
#[allow(clippy::unused_async)]
pub async fn music(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// gets the songbird client
/// this WILL PANIC if the client doesn't exist on the bot. is that a bad idea? maybe, i don't care
async fn get_client(ctx: &Context<'_>) -> Arc<Songbird> {
    if let Some(client) = songbird::get(ctx.serenity_context).await {
        client
    } else {
        println!("no songbird client exists, i will now cease to exist");
        // this is ignored because the bots about to crash, so this doesnt matter
        let _ = send_application_reply(*ctx, |r| r.content("Something has gone wrong internally. In fact, it's so bad that I'm going to crash after I send this message. Oops!")).await;
        panic!();
    }
}

async fn get_handler(
    ctx: &Context<'_>,
    guild_id: &GuildId,
    connect_to: &ChannelId,
) -> Result<Arc<Mutex<Call>>, crate::Error> {
    let manager = get_client(ctx).await;

    let handler_lock = if let Some(handler) = manager.get(*guild_id) {
        {
            let lock = handler.lock().await;
            if lock.current_channel().is_none() {
                drop(lock);
                let (lock, result) = manager.join(*guild_id, *connect_to).await;
                
                match result {
                    Ok(()) => lock,
                    Err(why) => return Err(why.into())
                }
            } else {
                drop(lock);
                handler
            }
        }
    } else {
        let (handler, result) = manager.join(*guild_id, *connect_to).await;

        {
            let mut lock = handler.lock().await;
            lock.add_global_event(
                songbird::Event::Periodic(Duration::from_secs(60), None),
                AutoLeave {
                    manager: manager.clone(),
                    guild: *guild_id,
                },
            );

            lock.add_global_event(
                songbird::Event::Track(songbird::TrackEvent::End),
                NowPlaying {
                    http: ctx.serenity_context().http.clone(),
                    manager: manager.clone(),
                    guild: *guild_id,
                },
            );
        }

        match result {
            Ok(()) => handler,
            Err(why) => return Err(why.into())
        }
    };

    Ok(handler_lock)
}

async fn get_color_from_thumbnail(metadata: &Metadata) -> Option<RGB<u8>> {
    match metadata.thumbnail.clone() {
        Some(t) => {
            if let Ok(response) = reqwest::get(t).await {
                response.bytes().await.map_or(None, |image_bytes|
                    image::load_from_memory(&image_bytes).map_or(None, |image| {
                        let pixels = image.to_rgb8();

                        color_thief::get_palette(&pixels, color_thief::ColorFormat::Rgb, 10, 2).map_or(None, |mut pallette| {
                            // sort by saturation
                            pallette.sort_by(|a, b| saturation_from_rgb(a.r, a.g, a.b).partial_cmp(&saturation_from_rgb(b.r, b.g, b.b)).expect("NaN snuck in, something has gone wrong with pallette sorting"));
                            pallette.reverse();
                            Some(pallette[0])
                        })
                    }))
            } else {
                None
            }
        }
        None => None,
    }
}

// dervied from https://donatbalipapp.medium.com/colours-maths-90346fb5abda
fn saturation_from_rgb(r: u8, g: u8, b: u8) -> f64 {
    let max_rgb = f64::from(r.max(g).max(b));
    let min_rgb = f64::from(r.min(g).min(b));
    let luminosity = 0.5 * (max_rgb + min_rgb);

    if luminosity < 1. {
        (max_rgb - min_rgb) / 1. - 2.0f64.mul_add(luminosity, -1.)
    } else {
        0.
    }
}

fn make_now_playing_embed<'_0>(
    embed: &'_0 mut CreateEmbed,
    metadata: &Metadata,
    color: Option<RGB<u8>>,
    requester: Option<&TrackRequester>,
) -> &'_0 mut CreateEmbed {
    embed
        .title("Now Playing:")
        .thumbnail(
            metadata
                .thumbnail
                .clone()
                .unwrap_or_else(|| "https://http.cat/404".to_string()),
        )
        .description(format!(
            "*{}*",
            metadata
                .source_url
                .clone()
                .unwrap_or_else(|| "https://http.cat/404".to_string())
        ))
        .field(
            "Title",
            metadata.title.clone().unwrap_or_else(|| "-".to_string()),
            true,
        )
        .field(
            "Artist",
            metadata.artist.clone().unwrap_or_else(|| "-".to_string()),
            true,
        )
        .timestamp(Utc::now());

    if let Some(color) = color {
        embed.color((color.r, color.g, color.b));
    }

    if let Some(requester) = requester {
        embed.footer(|f| {
            f.icon_url(requester.avatar_url.clone())
                .text(format!("Requested by {}", requester.name))
        });
    }

    embed
}

struct NowPlaying {
    http: Arc<Http>,
    manager: Arc<Songbird>,
    guild: GuildId,
}

#[async_trait]
impl EventHandler for NowPlaying {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        // nested if let hell
        if let EventContext::Track(_track_list) = ctx {
            let handler_lock = self.manager.get(self.guild)?;
            let handler = handler_lock.lock().await;
            let np = handler.queue().current()?;
            let channel_id = handler.current_channel()?;
            drop(handler);
            if let Channel::Guild(channel) = self.http.get_channel(channel_id.0).await.ok()? {
                let metadata = np.metadata();
                let color = get_color_from_thumbnail(metadata).await;
                let requester = np.typemap().read().await.get::<TrackRequester>().cloned();

                if let Err(why) = channel
                    .send_message(&self.http, |r| {
                        r.add_embed(|e| {
                            make_now_playing_embed(e, metadata, color, requester.as_ref())
                        })
                    })
                    .await
                {
                    println!("Error sending now playing message: {:?}", why);
                }
            }
        }

        None
    }
}

struct AutoLeave {
    manager: Arc<Songbird>,
    guild: GuildId,
}

#[async_trait]
impl EventHandler for AutoLeave {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        if let Some(handler_lock) = self.manager.get(self.guild) {
            let mut handler = handler_lock.lock().await;
            if handler.queue().is_empty() {
                let _dc = handler.leave().await;
                drop(handler);
            }
        }

        None
    }
}
