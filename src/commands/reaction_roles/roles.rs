use itertools::Itertools;
use poise::{
    send_application_reply,
    serenity_prelude::{
        ActionRowComponent, ButtonKind, CreateActionRow, CreateButton, EditMessage, Message,
        ReactionType, Role,
    },
    CreateReply,
};

use crate::{commands::reaction_roles::RoleButton, local_get, Context, Error, ID_REGEX};

#[poise::command(slash_command, subcommands("add", "remove"))]
#[allow(clippy::unused_async)]
pub async fn roles(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, ephemeral, guild_only)]
#[allow(clippy::too_many_lines)]
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
    if message.author.id != ctx.serenity_context().cache.current_user().id {
        send_application_reply(
            ctx,
            CreateReply::default().content(local_get(
                &ctx.data.translator,
                "commands_reactionroles_add_notsentbybot",
                locale,
            )),
        )
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
        let ActionRowComponent::Button(button) = component else {
            send_application_reply(
                ctx,
                CreateReply::default().content(local_get(
                    &ctx.data.translator,
                    "commands_reactionroles_roles_add_probablynoindex",
                    locale,
                )),
            )
            .await?;
            return Ok(());
        };

        let ButtonKind::NonLink {
            custom_id,
            style: _,
        } = button.data
        else {
            send_application_reply(
                ctx,
                CreateReply::default().content(local_get(
                    &ctx.data.translator,
                    "commands_reactionroles_roles_add_noindex",
                    locale,
                )),
            )
            .await?;
            return Ok(());
        };

        if !ID_REGEX.is_match(&custom_id) {
            send_application_reply(
                ctx,
                CreateReply::default().content(local_get(
                    &ctx.data.translator,
                    "commands_reactionroles_roles_add_noindex",
                    locale,
                )),
            )
            .await?;
            return Ok(());
        }

        let Some(label) = button.label else {
            send_application_reply(
                ctx,
                CreateReply::default().content(local_get(
                    &ctx.data.translator,
                    "commands_reactionroles_roles_add_noindex",
                    locale,
                )),
            )
            .await?;
            return Ok(());
        };

        buttons.push(RoleButton {
            id: custom_id,
            label,
            emoji: button.emoji,
        });
    }

    buttons.push(RoleButton {
        id: format!("rr:{}", role.id),
        label: name.unwrap_or(role.name),
        emoji,
    });

    message
        .edit(
            &ctx.serenity_context,
            EditMessage::new().components(
                buttons
                    .iter()
                    .chunks(5)
                    .into_iter()
                    .map(|c| {
                        CreateActionRow::Buttons(
                            c.map(|b| {
                                let mut bt = CreateButton::new(b.id.clone())
                                    .style(poise::serenity_prelude::ButtonStyle::Secondary)
                                    .label(b.label.clone());

                                if let Some(emoji) = b.emoji.clone() {
                                    bt = bt.emoji(emoji);
                                }

                                bt
                            })
                            .collect(),
                        )
                    })
                    .collect(),
            ),
        )
        .await?;

    send_application_reply(
        ctx,
        CreateReply::default().content(local_get(
            &ctx.data.translator,
            "commands_reactionroles_roles_add_success",
            locale,
        )),
    )
    .await?;

    Ok(())
}

#[poise::command(slash_command, ephemeral, guild_only)]
#[allow(clippy::too_many_lines)]
async fn remove(ctx: Context<'_>, role: Role, mut message: Message) -> Result<(), Error> {
    let locale = ctx
        .locale()
        .expect("locale should always be available for slash commands");

    if message.author.id != ctx.serenity_context().cache.current_user().id {
        send_application_reply(
            ctx,
            CreateReply::default().content(local_get(
                &ctx.data.translator,
                "commands_reactionroles_remove_notsentbybot",
                locale,
            )),
        )
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
        let ActionRowComponent::Button(button) = component else {
            send_application_reply(
                ctx,
                CreateReply::default().content(local_get(
                    &ctx.data.translator,
                    "commands_reactionroles_roles_remove_probablynoindex",
                    locale,
                )),
            )
            .await?;
            return Ok(());
        };

        let ButtonKind::NonLink {
            custom_id,
            style: _,
        } = button.data
        else {
            send_application_reply(
                ctx,
                CreateReply::default().content(local_get(
                    &ctx.data.translator,
                    "commands_reactionroles_roles_remove_noindex",
                    locale,
                )),
            )
            .await?;
            return Ok(());
        };

        if !ID_REGEX.is_match(&custom_id) {
            send_application_reply(
                ctx,
                CreateReply::default().content(local_get(
                    &ctx.data.translator,
                    "commands_reactionroles_roles_remove_noindex",
                    locale,
                )),
            )
            .await?;
            return Ok(());
        }

        let Some(label) = button.label else {
            send_application_reply(
                ctx,
                CreateReply::default().content(local_get(
                    &ctx.data.translator,
                    "commands_reactionroles_roles_remove_noindex",
                    locale,
                )),
            )
            .await?;
            return Ok(());
        };

        buttons.push(RoleButton {
            id: custom_id,
            label,
            emoji: button.emoji,
        });
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
            role.id != parsed
        } else {
            false
        }
    });

    message
        .edit(
            &ctx.serenity_context,
            EditMessage::new().components(
                buttons
                    .iter()
                    .chunks(5)
                    .into_iter()
                    .map(|c| {
                        CreateActionRow::Buttons(
                            c.map(|b| {
                                let mut bt = CreateButton::new(b.id.clone())
                                    .style(poise::serenity_prelude::ButtonStyle::Secondary)
                                    .label(b.label.clone());

                                if let Some(emoji) = b.emoji.clone() {
                                    bt = bt.emoji(emoji);
                                }

                                bt
                            })
                            .collect(),
                        )
                    })
                    .collect(),
            ),
        )
        .await?;

    send_application_reply(
        ctx,
        CreateReply::default().content(local_get(
            &ctx.data.translator,
            "commands_reactionroles_roles_remove_success",
            locale,
        )),
    )
    .await?;

    Ok(())
}
