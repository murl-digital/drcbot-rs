use mongodb::{
    bson::doc,
    results::{InsertOneResult, UpdateResult},
    Client,
};
use poise::serenity_prelude::{ChannelId, GuildId, MessageId};
use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ReactionRolesIndex {
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub messages: Vec<ReactionRolesMessage>,
}

#[derive(Serialize, Deserialize)]
pub struct ReactionRolesMessage {
    pub title: String,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
}

pub async fn get_index(
    client: &Client,
    guild_id: &GuildId,
) -> Result<Option<ReactionRolesIndex>, mongodb::error::Error> {
    let db = client.database("placeholder");
    let collection = db.collection("reactionRolesIndices");
    let filter = doc! { "guild_id":  guild_id.0.to_string() };

    collection.find_one(filter, None).await
}

pub async fn save_index(
    client: &Client,
    guild_id: GuildId,
    channel_id: ChannelId,
    message_id: MessageId,
) -> Result<InsertOneResult, mongodb::error::Error> {
    let db = client.database("placeholder");
    let collection = db.collection("reactionRolesIndices");

    collection
        .insert_one(
            ReactionRolesIndex {
                guild_id,
                channel_id,
                message_id,
                messages: vec![],
            },
            None,
        )
        .await
}

pub async fn replace_index(
    client: &Client,
    index: &ReactionRolesIndex,
) -> Result<UpdateResult, mongodb::error::Error> {
    let db = client.database("placeholder");
    let collection = db.collection::<ReactionRolesIndex>("reactionRolesIndices");
    let query = doc! { "guild_id": index.guild_id.0.to_string() };

    collection.replace_one(query, index, None).await
}
