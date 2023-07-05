use crate::commands::reaction_roles::update_index;
use crate::{local_get, Context, Error};
use poise::serenity_prelude::{GuildChannel, Message};
use poise::{send_application_reply, Modal};

#[poise::command(slash_command, subcommands("init", "add", "list", "remove"))]
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

#[poise::command(slash_command, ephemeral)]
async fn init(ctx: Context<'_>, channel: GuildChannel) -> Result<(), Error> {
    let locale = ctx
        .locale()
        .expect("locale should always be available for slash commands");
    if ctx
        .data
        .database
        .get_index(&channel.guild_id)
        .await?
        .is_some()
    {
        send_application_reply(ctx, |r| {
            r.content(local_get(
                &ctx.data.translator,
                "commands_reactionroles_init_exists",
                locale,
            ))
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

    ctx.data
        .database
        .save_index(channel.guild_id, channel.id, message.id)
        .await?;

    send_application_reply(ctx, |r| {
        r.content(local_get(
            &ctx.data.translator,
            "commands_reactionroles_init_success",
            locale,
        ))
        .ephemeral(true)
    })
    .await?;

    Ok(())
}

#[poise::command(slash_command, ephemeral)]
async fn add(ctx: Context<'_>, channel: GuildChannel, infocard: bool) -> Result<(), Error> {
    let locale = ctx
        .locale()
        .expect("locale should always be available for slash commands");
    if let Some(mut index) = ctx.data.database.get_index(&channel.guild_id).await? {
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

            ctx.data.database.replace_index(&index).await?;

            update_index(&ctx, &index).await?;

            send_application_reply(ctx, |r| {
                r.content(local_get(
                    &ctx.data.translator,
                    "commands_reactionroles_add_success",
                    locale,
                ))
            })
            .await?;
        }
    } else {
        send_application_reply(ctx, |r| {
            r.content(local_get(
                &ctx.data.translator,
                "commands_reactionroles_add_noindex",
                locale,
            ))
            .ephemeral(true)
        })
        .await?;
    }

    Ok(())
}

#[poise::command(slash_command, ephemeral)]
async fn remove(ctx: Context<'_>, message: Message) -> Result<(), Error> {
    let locale = ctx
        .locale()
        .expect("locale should always be available for slash commands");

    if let Some(guild_id) = message.guild_id {
        if let Some(mut index) = ctx.data.database.get_index(&guild_id).await? {
            if index.messages.iter().any(|m| m.message_id == message.id) {
                message.delete(&ctx.serenity_context.http).await?;

                index.messages.retain(|m| m.message_id != message.id);

                ctx.data.database.replace_index(&index).await?;

                update_index(&ctx, &index).await?;

                send_application_reply(ctx, |r| {
                    r.content(local_get(
                        &ctx.data.translator,
                        "commands_reactionroles_remove_success",
                        locale,
                    ))
                })
                .await?;
            }
        } else {
            send_application_reply(ctx, |r| {
                r.content(local_get(
                    &ctx.data.translator,
                    "commands_reactionroles_remove_noindex",
                    locale,
                ))
                .ephemeral(true)
            })
            .await?;
        }
    }

    Ok(())
}

#[poise::command(slash_command, ephemeral)]
async fn list(ctx: Context<'_>) -> Result<(), Error> {
    if let Some(index) = ctx
        .data
        .database
        .get_index(&ctx.guild_id().expect("we should be in a guild right now"))
        .await?
    {
        send_application_reply(ctx, |r| {
            r.embed(|e| {
                e.field(
                    "Index",
                    format!(
                        "https://discord.com/channels/{}/{}/{}",
                        index.guild_id.as_u64(),
                        index.channel_id.as_u64(),
                        index.message_id.as_u64()
                    ),
                    false,
                );

                e.fields(index.messages.iter().map(|m| {
                    (
                        &m.title,
                        format!(
                            "https://discord.com/channels/{}/{}/{}",
                            index.guild_id.as_u64(),
                            m.channel_id.as_u64(),
                            m.message_id.as_u64()
                        ),
                        true,
                    )
                }))
            })
        })
        .await?;
    }

    Ok(())
}
