use serde_json::json;
pub use sqlx::{
    mysql::{MySqlPoolOptions, MySqlQueryResult},
    pool::PoolConnection,
    types::{time::OffsetDateTime, Json},
    Error,
    FromRow,
    MySql,
    Pool,
};
use youtube_dl::{Playlist, SingleVideo};

pub struct PlaylistWrapper(Playlist);

impl From<Playlist> for PlaylistWrapper {
    fn from(playlist: Playlist) -> Self {
        Self(playlist)
    }
}

#[derive(Debug, FromRow)]
pub struct Channel {
    pub id: String,
    pub name: Option<String>,
    pub updated_at: Option<OffsetDateTime>,
    pub video_count: u64,
}

impl TryFrom<SingleVideo> for Channel {
    type Error = ();

    fn try_from(single_video: SingleVideo) -> Result<Self, Self::Error> {
        if let Some(id) = single_video.channel_id {
            let channel = Self {
                id,
                name: single_video.channel,
                updated_at: None,
                video_count: 1,
            };

            Ok(channel)
        } else {
            Err(())
        }
    }
}

impl TryFrom<Playlist> for Channel {
    type Error = ();

    fn try_from(playlist: Playlist) -> Result<Self, Self::Error> {
        if let Some(id) = playlist.uploader_id {
            let channel = Self {
                id,
                name: playlist.uploader.to_owned(),
                updated_at: Some(OffsetDateTime::now_utc()),
                video_count: playlist.entries.iter().flatten().count() as u64,
            };

            Ok(channel)
        } else {
            Err(())
        }
    }
}

#[derive(Debug, FromRow)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub tags: Json<Option<Vec<String>>>,
    pub channel_id: String,
    pub channel_name: Option<String>,
}

impl TryFrom<SingleVideo> for Video {
    type Error = ();

    fn try_from(single_video: SingleVideo) -> Result<Self, Self::Error> {
        let Some(channel_id) = single_video.channel_id else {
            return Err(());
        };

        let Some(title) = single_video.title else {
            return Err(());
        };

        Ok(Self {
            id: single_video.id.to_owned(),
            title,
            tags: get_tags(single_video.tags, None),
            channel_id,
            channel_name: single_video.channel,
        })
    }
}

impl From<PlaylistWrapper> for Vec<Video> {
    fn from(playlist: PlaylistWrapper) -> Self {
        playlist
            .0
            .entries
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .filter_map(|single_video| {
                let Some(channel_id) = &playlist.0.uploader_id else {
                    return None;
                };

                let Some(title) = single_video.title else {
                    return None;
                };

                Some(Video {
                    id: single_video.id.to_owned(),
                    title,
                    tags: get_tags(single_video.tags, None),
                    channel_id: channel_id.to_owned(),
                    channel_name: single_video.channel,
                })
            })
            .collect()
    }
}

pub async fn get_all_videos(conn: &mut PoolConnection<MySql>) -> Result<Vec<Video>, Error> {
    sqlx::query_as::<_, Video>("SELECT * FROM videos")
        .fetch_all(conn)
        .await
}

pub async fn get_oldest_channels(
    conn: &mut PoolConnection<MySql>,
    limit: u32,
) -> Result<Vec<Channel>, Error> {
    sqlx::query_as::<_, Channel>(
        r#"
            SELECT *
            FROM channels
            ORDER BY updated_at
            LIMIT ?
        "#,
    )
    .bind(limit)
    .fetch_all(conn)
    .await
}

pub async fn get_biggest_channels(
    conn: &mut PoolConnection<MySql>,
    limit: u32,
) -> Result<Vec<Channel>, Error> {
    sqlx::query_as::<_, Channel>(
        r#"
            SELECT *
            FROM channels
            WHERE
                YEAR(updated_at) < 3000
                AND
                video_count > 0
                AND
                video_count < 10000
            ORDER BY video_count DESC
            LIMIT ?
        "#,
    )
    .bind(limit)
    .fetch_all(conn)
    .await
}

pub async fn get_smallest_channels(
    conn: &mut PoolConnection<MySql>,
    limit: u32,
) -> Result<Vec<Channel>, Error> {
    sqlx::query_as::<_, Channel>(
        r#"
            SELECT *
            FROM channels
            WHERE
                YEAR(updated_at) < 3000
                AND
                video_count > 0
                AND
                video_count < 10000
            ORDER BY video_count
            LIMIT ?
        "#,
    )
    .bind(limit)
    .fetch_all(conn)
    .await
}

pub async fn get_tagless_videos(
    conn: &mut PoolConnection<MySql>,
    limit: u32,
) -> Result<Vec<Video>, Error> {
    sqlx::query_as::<_, Video>(
        r#"
            SELECT *
            FROM videos
            WHERE tags LIKE 'null'
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
            INSERT IGNORE INTO channels (id, name, updated_at, video_count)
            VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(&channel.id)
    .bind(&channel.name)
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
            INSERT INTO channels (id, name, updated_at, video_count)
            VALUES (?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE id = VALUES(id), name = VALUES(name), updated_at = VALUES(updated_at), video_count = VALUES(video_count)
        "#,
    )
    .bind(&channel.id)
    .bind(&channel.name)
    .bind(channel.updated_at)
    .bind(channel.video_count)
    .execute(conn)
    .await
}

pub async fn upsert_video(
    conn: &mut PoolConnection<MySql>,
    video: Video,
) -> Result<MySqlQueryResult, Error> {
    let sql = if video.tags.is_none() {
        r#"
            INSERT INTO videos (id, title, tags, channel_id, channel_name)
            VALUES (?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE title = VALUES(title), channel_id = VALUES(channel_id), channel_name = VALUES(channel_name)
        "#
    } else {
        r#"
            INSERT INTO videos (id, title, tags, channel_id, channel_name)
            VALUES (?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE title = VALUES(title), tags = VALUES(tags), channel_id = VALUES(channel_id), channel_name = VALUES(channel_name)
        "#
    };

    sqlx::query(sql)
        .bind(&video.id)
        .bind(&video.title)
        .bind(&json!(video.tags))
        .bind(&video.channel_id)
        .bind(&video.channel_name)
        .execute(conn)
        .await
}

pub fn get_tags(
    tags: Option<Vec<Option<String>>>,
    default: Option<Vec<String>>,
) -> Json<Option<Vec<String>>> {
    if let Some(tags) = tags {
        let tags = tags.into_iter().flatten().collect();
        Json(Some(tags))
    } else if let Some(default) = default {
        Json(Some(default))
    } else {
        Json(None)
    }
}
