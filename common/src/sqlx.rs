use serde_json::json;
use sqlx::{
    mysql::MySqlQueryResult,
    pool::PoolConnection,
    query,
    types::time::PrimitiveDateTime,
    Error,
    MySql,
};

pub struct Channel {
    pub id: String,
    pub handle: Option<String>,
    pub updated_at: Option<PrimitiveDateTime>,
}

pub struct Video {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub channel_id: String,
    pub channel_handle: Option<String>,
}

pub async fn upsert_channel(
    conn: &mut PoolConnection<MySql>,
    channel: Channel,
) -> Result<MySqlQueryResult, Error> {
    query!(
        r#"
            INSERT INTO channels (id, handle, updated_at) VALUES (?, ?, ?)
            ON DUPLICATE KEY UPDATE id = ?, handle = ?, updated_at = ?
        "#,
        channel.id,
        channel.handle,
        channel.updated_at,
        channel.id,
        channel.handle,
        channel.updated_at,
    )
    .execute(conn)
    .await
}

pub async fn upsert_video(
    conn: &mut PoolConnection<MySql>,
    video: Video,
) -> Result<MySqlQueryResult, Error> {
    query!(
        r#"
            INSERT INTO videos (id, title, tags, channel_id, channel_handle) VALUES (?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE id = ?, title = ?, tags = ?, channel_id = ?, channel_handle = ?
        "#,
        video.id,
        video.title,
        json!(video.tags),
        video.channel_id,
        video.channel_handle,
        video.id,
        video.title,
        json!(video.tags),
        video.channel_id,
        video.channel_handle,
    )
    .execute(conn)
    .await
}
