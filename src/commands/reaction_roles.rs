use poise::serenity_prelude::{CreateEmbed, EditMessage, EmbedMessageBuilding, MessageBuilder, ReactionType};

use crate::commands::reaction_roles::messages::messages;
use crate::commands::reaction_roles::roles::roles;
use crate::{Context, Error};

mod messages;
mod roles;

struct RoleButton {
    id: String,
    label: String,
    emoji: Option<ReactionType>,
}

#[poise::command(
    slash_command,
    required_permissions = "MANAGE_MESSAGES",
    subcommands("messages", "roles"),
    guild_only,
    rename = "reaction-roles"
)]
#[allow(clippy::unused_async)]
pub async fn reaction_roles(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

async fn update_index(
    ctx: &Context<'_>,
    index: &crate::data::ReactionRolesIndex,
) -> Result<(), Error> {
    let mut index_message = ctx
        .serenity_context
        .http
        .get_message(index.channel_id, index.message_id)
        .await?;

    let mut embed = CreateEmbed::default()
    .title("Reaction Roles Index")
        .description("Click a link to get sent to the associated category")
        .timestamp(chrono::Utc::now());

        let mut field_value = MessageBuilder::new();
        for link in index.messages.iter().map(|rm| {
            MessageBuilder::new()
                .push_named_link(
                    rm.title.clone(),
                    format!(
                        "https://discord.com/channels/{}/{}/{}",
                        index.guild_id,
                        rm.channel_id,
                        rm.message_id
                    ),
                )
                .build()
        }) {
            field_value.push_quote_line(link);
        }

        embed = embed.field("Links", field_value.build(), false);

    index_message
        .edit(ctx.serenity_context, EditMessage::default().embed(embed))
        .await?;

    Ok(())
}
