use crate::commands::reaction_roles::update_index;
use crate::{Context, Error};
use poise::serenity_prelude::GuildChannel;
use poise::{send_application_reply, Modal};

#[poise::command(slash_command, subcommands("init", "add"))]
pub async fn messages(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[derive(Modal)]
#[name = "Create Message"]
struct MessageCreate {
    #[name = "Title"]
    title: String,
    #[name = "Description"]
    #[paragraph]
    description: String,
}

#[poise::command(slash_command)]
async fn init(ctx: Context<'_>, channel: GuildChannel) -> Result<(), Error> {
    if crate::data::get_index(&ctx.data.client, &channel.guild_id)
        .await?
        .is_some()
    {
        send_application_reply(ctx, |r| {
            r.content("You already have an index in this guild")
                .ephemeral(true)
        })
        .await?;

        return Ok(());
    }

    let message = channel
        .send_message(ctx.serenity_context, |m| {
            m.add_embed(|e| {
                e.title("Reaction Roles Index")
                    .description("Click a link to get sent to the associated category")
                    .timestamp(chrono::Utc::now())
            })
        })
        .await?;

    crate::data::save_index(&ctx.data.client, channel.guild_id, channel.id, message.id).await?;

    send_application_reply(ctx, |r| {
        r.content("Index successfully created").ephemeral(true)
    })
    .await?;

    Ok(())
}

#[poise::command(slash_command)]
async fn add(ctx: Context<'_>, channel: GuildChannel, infocard: bool) -> Result<(), Error> {
    if let Some(mut index) = crate::data::get_index(&ctx.data.client, &channel.guild_id).await? {
        if let Some(message_data) = MessageCreate::execute(ctx).await? {
            let message = channel
                .send_message(ctx.serenity_context, |m| {
                    m.add_embed(|e| {
                        e.title(message_data.title.clone())
                            .description(message_data.description);

                        if infocard {
                            e.footer(|f| {
                                f.text("Click one of the buttons to get (or lose) a role")
                            });
                        }

                        e
                    })
                })
                .await?;

            index.messages.push(crate::data::ReactionRolesMessage {
                title: message_data.title,
                message_id: message.id,
                channel_id: channel.id,
            });

            crate::data::replace_index(&ctx.data.client, &index).await?;

            update_index(&ctx, &index).await?;

            send_application_reply(ctx, |r| r.content("Message created!")).await?;
        }
    } else {
        send_application_reply(ctx, |r| {
            r.content("Looks like you don't have an index, cope.")
                .ephemeral(true)
        })
        .await?;
    }

    Ok(())
}
