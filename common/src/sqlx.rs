use serde_json::json;
use sqlx::types::Json;
pub use sqlx::{
    mysql::{MySqlPoolOptions, MySqlQueryResult},
    pool::PoolConnection,
    types::time::PrimitiveDateTime,
    Error,
    FromRow,
    MySql,
    Pool,
};
use youtube_dl::SingleVideo;

#[derive(Debug, FromRow)]
pub struct Channel {
    pub id: String,
    pub handle: Option<String>,
    pub updated_at: Option<PrimitiveDateTime>,
    pub video_count: i32,
}

#[derive(Debug, FromRow)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub tags: Json<Vec<String>>,
    pub channel_id: String,
    pub channel_handle: Option<String>,
    pub channel_name: Option<String>,
}

pub async fn get_all_videos(conn: &mut PoolConnection<MySql>) -> Result<Vec<Video>, Error> {
    sqlx::query_as::<_, Video>(
        r#"
            SELECT *
            FROM videos
            ORDER BY channel_handle
        "#,
    )
    .fetch_all(conn)
    .await
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
    .bind(limit)
    .fetch_all(conn)
    .await
}

pub async fn insert_channel(
    conn: &mut PoolConnection<MySql>,
    channel: Channel,
) -> Result<MySqlQueryResult, Error> {
    sqlx::query(
        r#"
            INSERT IGNORE INTO channels (id, handle, updated_at, video_count)
            VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(&channel.id)
    .bind(&channel.handle)
    .bind(channel.updated_at)
    .bind(channel.video_count)
    .execute(conn)
    .await
}

pub async fn upsert_channel(
    conn: &mut PoolConnection<MySql>,
    channel: Channel,
) -> Result<MySqlQueryResult, Error> {
    sqlx::query(
        r#"
            INSERT INTO channels (id, handle, updated_at, video_count)
            VALUES (?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE id = ?, handle = ?, updated_at = ?, video_count = ?
        "#,
    )
    .bind(&channel.id)
    .bind(&channel.handle)
    .bind(channel.updated_at)
    .bind(channel.video_count)
    .bind(&channel.id)
    .bind(&channel.handle)
    .bind(channel.updated_at)
    .bind(channel.video_count)
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

pub fn get_tags(single_video: &SingleVideo) -> Json<Vec<String>> {
    let tags = single_video
        .tags
        .clone()
        .map(|opt_tags| opt_tags.into_iter().flatten().collect())
        .unwrap_or_else(Vec::new);

    Json(tags)
}
