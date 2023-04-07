use crate::{Context, Error, ID_REGEX};
use itertools::Itertools;
use poise::serenity_prelude::{
    ActionRowComponent, EmbedMessageBuilding, GuildChannel, Message, MessageBuilder, ReactionType,
    Role,
};
use poise::{send_application_reply, Modal};

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
pub async fn init(ctx: Context<'_>, channel: GuildChannel) -> Result<(), Error> {
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
pub async fn add_message(
    ctx: Context<'_>,
    channel: GuildChannel,
    infocard: bool,
) -> Result<(), Error> {
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

struct RoleButton {
    id: String,
    label: String,
    emoji: Option<ReactionType>,
}

#[poise::command(slash_command, ephemeral, guild_only)]
pub async fn add_role(
    ctx: Context<'_>,
    role: Role,
    emoji: Option<ReactionType>,
    name: Option<String>,
    mut message: Message,
) -> Result<(), Error> {
    if message.author.id != ctx.serenity_context().cache.current_user_id() {
        send_application_reply(ctx, |r| {
            r.content("This message wasn't sent by me, are you sure this is the right one?")
        })
        .await?;

        return Ok(());
    }

    let components: Vec<ActionRowComponent> = message
        .components
        .iter()
        .flat_map(|r| r.components.clone())
        .collect();

    let mut buttons = vec![];

    for component in components {
        let button = match component {
            ActionRowComponent::Button(b) => b,
            _ => {
                send_application_reply(ctx, |r| r.content("This message probably isn't a reactionroles message. Reactionroles messages only have buttons.")).await?;
                return Ok(());
            }
        };

        let id = match button.custom_id {
            Some(id) => id,
            None => {
                send_application_reply(ctx, |r| r.content("This message isn't a reaction roles message. Double check that you're passing the right one.")).await?;
                return Ok(());
            }
        };

        if !ID_REGEX.is_match(&id) {
            send_application_reply(ctx, |r| r.content("This message isn't a reaction roles message. Double check that you're passing the right one.")).await?;
            return Ok(());
        }

        let label = match button.label {
            Some(l) => l,
            None => {
                send_application_reply(ctx, |r| r.content("This message isn't a reaction roles message. Double check that you're passing the right one.")).await?;
                return Ok(());
            }
        };

        buttons.push(RoleButton {
            id,
            label,
            emoji: button.emoji,
        })
    }

    buttons.push(RoleButton {
        id: format!("rr:{}", role.id),
        label: name.unwrap_or(role.name),
        emoji,
    });

    message
        .edit(&ctx.serenity_context, |e| {
            e.components(|c| {
                for row in &buttons.iter().chunks(5) {
                    c.create_action_row(|r| {
                        for b in row {
                            r.create_button(|bt| {
                                bt.style(poise::serenity_prelude::ButtonStyle::Secondary)
                                    .custom_id(b.id.clone())
                                    .label(b.label.clone());

                                if let Some(emoji) = b.emoji.clone() {
                                    bt.emoji(emoji);
                                }

                                bt
                            });
                        }

                        r
                    });
                }

                c
            })
        })
        .await?;

    send_application_reply(ctx, |r| r.content("Role added")).await?;

    Ok(())
}

async fn update_index(
    ctx: &Context<'_>,
    index: &crate::data::ReactionRolesIndex,
) -> Result<(), Error> {
    let mut index_message = ctx
        .serenity_context
        .http
        .get_message(index.channel_id.0, index.message_id.0)
        .await?;

    index_message
        .edit(ctx.serenity_context, |m| {
            m.embed(|e| {
                e.title("Reaction Roles Index")
                    .description("Click a link to get sent to the associated category")
                    .timestamp(chrono::Utc::now());

                let mut field_value = MessageBuilder::new();
                for link in index.messages.iter().map(|rm| {
                    MessageBuilder::new()
                        .push_named_link(
                            rm.title.clone(),
                            format!(
                                "https://discord.com/channels/{}/{}/{}",
                                index.guild_id.as_u64(),
                                rm.channel_id.as_u64(),
                                rm.message_id.as_u64()
                            ),
                        )
                        .build()
                }) {
                    field_value.push_quote_line(link);
                }

                e.field("Links", field_value.build(), false)
            })
        })
        .await?;

    Ok(())
}
