use itertools::Itertools;
use poise::{
    send_application_reply,
    serenity_prelude::{ActionRowComponent, Message, ReactionType, Role},
};

use crate::{commands::reaction_roles::RoleButton, local_get, Context, Error, ID_REGEX};

#[poise::command(slash_command, subcommands("add", "remove"))]
pub async fn roles(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, ephemeral, guild_only)]
async fn add(
    ctx: Context<'_>,
    role: Role,
    emoji: Option<ReactionType>,
    name: Option<String>,
    mut message: Message,
) -> Result<(), Error> {
    let locale = ctx
        .locale()
        .expect("locale should always be available for slash commands");
    if message.author.id != ctx.serenity_context().cache.current_user_id() {
        send_application_reply(ctx, |r| {
            r.content(local_get(
                &ctx.data.translator,
                "commands_reactionroles_add_notsentbybot",
                locale,
            ))
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
                send_application_reply(ctx, |r| {
                    r.content(local_get(
                        &ctx.data.translator,
                        "commands_reactionroles_roles_add_probablynoindex",
                        locale,
                    ))
                })
                .await?;
                return Ok(());
            }
        };

        let id = match button.custom_id {
            Some(id) => id,
            None => {
                send_application_reply(ctx, |r| {
                    r.content(local_get(
                        &ctx.data.translator,
                        "commands_reactionroles_roles_add_noindex",
                        locale,
                    ))
                })
                .await?;
                return Ok(());
            }
        };

        if !ID_REGEX.is_match(&id) {
            send_application_reply(ctx, |r| {
                r.content(local_get(
                    &ctx.data.translator,
                    "commands_reactionroles_roles_add_noindex",
                    locale,
                ))
            })
            .await?;
            return Ok(());
        }

        let label = match button.label {
            Some(l) => l,
            None => {
                send_application_reply(ctx, |r| {
                    r.content(local_get(
                        &ctx.data.translator,
                        "commands_reactionroles_roles_add_noindex",
                        locale,
                    ))
                })
                .await?;
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

    send_application_reply(ctx, |r| {
        r.content(local_get(
            &ctx.data.translator,
            "commands_reactionroles_roles_add_success",
            locale,
        ))
    })
    .await?;

    Ok(())
}

#[poise::command(slash_command, ephemeral, guild_only)]
async fn remove(ctx: Context<'_>, role: Role, mut message: Message) -> Result<(), Error> {
    let locale = ctx
        .locale()
        .expect("locale should always be available for slash commands");

    if message.author.id != ctx.serenity_context().cache.current_user_id() {
        send_application_reply(ctx, |r| {
            r.content(local_get(
                &ctx.data.translator,
                "commands_reactionroles_remove_notsentbybot",
                locale,
            ))
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
                send_application_reply(ctx, |r| {
                    r.content(local_get(
                        &ctx.data.translator,
                        "commands_reactionroles_roles_remove_probablynoindex",
                        locale,
                    ))
                })
                .await?;
                return Ok(());
            }
        };

        let id = match button.custom_id {
            Some(id) => id,
            None => {
                send_application_reply(ctx, |r| {
                    r.content(local_get(
                        &ctx.data.translator,
                        "commands_reactionroles_roles_remove_noindex",
                        locale,
                    ))
                })
                .await?;
                return Ok(());
            }
        };

        if !ID_REGEX.is_match(&id) {
            send_application_reply(ctx, |r| {
                r.content(local_get(
                    &ctx.data.translator,
                    "commands_reactionroles_roles_remove_noindex",
                    locale,
                ))
            })
            .await?;
            return Ok(());
        }

        let label = match button.label {
            Some(l) => l,
            None => {
                send_application_reply(ctx, |r| {
                    r.content(local_get(
                        &ctx.data.translator,
                        "commands_reactionroles_roles_remove_noindex",
                        locale,
                    ))
                })
                .await?;
                return Ok(());
            }
        };

        buttons.push(RoleButton {
            id,
            label,
            emoji: button.emoji,
        })
    }

    buttons.retain(|b| {
        let captures = ID_REGEX
            .captures(&b.id)
            .expect("id regex should get captures");
        if let Ok(parsed) = captures
            .get(1)
            .expect("there should always be at least 1 capture group")
            .as_str()
            .parse::<u64>()
        {
            role.id.0 != parsed
        } else {
            false
        }
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

        send_application_reply(ctx, |r| {
            r.content(local_get(
                &ctx.data.translator,
                "commands_reactionroles_roles_remove_success",
                locale,
            ))
        })
        .await?;

    Ok(())
}
