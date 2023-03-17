use serde_json::json;
pub use sqlx::{
    mysql::{MySqlPoolOptions, MySqlQueryResult},
    pool::PoolConnection,
    types::time::PrimitiveDateTime,
    Error,
    FromRow,
    MySql,
    Pool,
};

#[derive(Debug, FromRow)]
pub struct Channel {
    pub id: String,
    pub handle: Option<String>,
    pub updated_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, FromRow)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub channel_id: String,
    pub channel_handle: Option<String>,
}

pub async fn get_oldest_channels(
    conn: &mut PoolConnection<MySql>,
    limit: u8,
) -> Result<Vec<Channel>, Error> {
    sqlx::query_as::<_, Channel>(
        r#"
            SELECT *
            FROM channels
            ORDER BY updated_at, handle
            LIMIT ?
        "#,
    )
    .bind(&limit)
    .fetch_all(conn)
    .await
}

pub async fn upsert_channel(
    conn: &mut PoolConnection<MySql>,
    channel: Channel,
) -> Result<MySqlQueryResult, Error> {
    sqlx::query(
        r#"
            INSERT INTO channels (id, handle, updated_at)
            VALUES (?, ?, ?)
            ON DUPLICATE KEY UPDATE id = ?, handle = ?, updated_at = ?
        "#,
    )
    .bind(&channel.id)
    .bind(&channel.handle)
    .bind(&channel.updated_at)
    .bind(&channel.id)
    .bind(&channel.handle)
    .bind(&channel.updated_at)
    .execute(conn)
    .await
}

pub async fn upsert_video(
    conn: &mut PoolConnection<MySql>,
    video: Video,
) -> Result<MySqlQueryResult, Error> {
    sqlx::query(
        r#"
            INSERT INTO videos (id, title, tags, channel_id, channel_handle)
            VALUES (?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE id = ?, title = ?, tags = ?, channel_id = ?, channel_handle = ?
        "#,
    )
    .bind(&video.id)
    .bind(&video.title)
    .bind(&json!(video.tags))
    .bind(&video.channel_id)
    .bind(&video.channel_handle)
    .bind(&video.id)
    .bind(&video.title)
    .bind(&json!(video.tags))
    .bind(&video.channel_id)
    .bind(&video.channel_handle)
    .execute(conn)
    .await
}
