use itertools::Itertools;
use poise::{
    send_application_reply,
    serenity_prelude::{ActionRowComponent, Message, ReactionType, Role},
};

use crate::{commands::reaction_roles::RoleButton, Context, Error, ID_REGEX};

#[poise::command(slash_command, subcommands("add"))]
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
