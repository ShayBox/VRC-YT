use std::{path::PathBuf, time::SystemTime};

use anyhow::{bail, Result};
use clap::{Parser, ValueEnum};
use common::{
    sqlx::{
        get_oldest_channels,
        upsert_channel,
        upsert_video,
        Channel,
        MySql,
        MySqlPoolOptions,
        Pool,
        PoolConnection,
        Video,
    },
    youtube_dl::{get_playlist, get_tags, get_youtube_dl_path},
};
use dotenvy_macro::dotenv;

#[derive(Clone, Debug, ValueEnum)]
enum Mode {
    Add,
    Old,
}

#[derive(Clone, Debug, Parser)]
#[command()]
struct Args {
    #[arg(value_enum, short, long)]
    mode: Mode,

    #[arg(short, long)]
    channel: Option<String>,

    #[arg(short, long, default_value_t = 1)]
    limit: u8,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let ytdl = get_youtube_dl_path().await?;
    let pool = MySqlPoolOptions::new()
        .connect(dotenv!("DATABASE_URL"))
        .await?;

    match args.mode {
        Mode::Add => add(pool, ytdl, args.channel).await,
        Mode::Old => old(pool, ytdl, args.limit).await,
    }
}

async fn add(pool: Pool<MySql>, ytdl: PathBuf, channel: Option<String>) -> Result<()> {
    let mut conn = pool.acquire().await?;
    let Some(channel) = channel else {
        bail!("Channel required");
    };

    get(&mut conn, &ytdl, channel).await?;

    Ok(())
}

async fn old(pool: Pool<MySql>, ytdl: PathBuf, limit: u8) -> Result<()> {
    let mut conn = pool.acquire().await?;

    let channels = get_oldest_channels(&mut conn, limit).await?;
    for channel in channels {
        let channel = channel.handle.unwrap_or(channel.id);
        get(&mut conn, &ytdl, channel).await?;
    }

    Ok(())
}

async fn get(conn: &mut PoolConnection<MySql>, ytdl: &PathBuf, channel: String) -> Result<()> {
    let url = format!("https://youtube.com/{channel}/videos",);
    println!("Fetching {url}");

    let playlist = get_playlist(ytdl, &url)?;
    let Some(single_videos) = playlist.entries else {
        bail!("No videos found");
    };

    let Some(single_video) = single_videos.first() else {
        bail!("No video found");
    };

    if let Some(channel_id) = &single_video.channel_id {
        let channel = Channel {
            id: channel_id.to_owned(),
            handle: single_video.uploader_id.to_owned(),
            updated_at: Some(SystemTime::now().into()),
        };

        let _ = upsert_channel(conn, channel).await;
    }

    for single_video in single_videos {
        let Some(channel_id) = &single_video.channel_id else {
            continue;
        };

        let video = Video {
            id: single_video.id.to_owned(),
            title: single_video.title.to_owned(),
            tags: get_tags(&single_video),
            channel_id: channel_id.to_owned(),
            channel_handle: single_video.uploader_id.to_owned(),
        };

        println!("Video: {}", video.title);
        let _ = upsert_video(conn, video).await;
    }

    Ok(())
}
