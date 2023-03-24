use std::{fs::File, io::Write, path::PathBuf, time::SystemTime};

use anyhow::{bail, Result};
use clap::{Parser, ValueEnum};
use common::{
    sqlx::{
        get_all_videos,
        get_oldest_channels,
        get_tags,
        upsert_channel,
        upsert_video,
        Channel,
        MySql,
        MySqlPoolOptions,
        Pool,
        PoolConnection,
        Video,
    },
    youtube_dl::{get_playlist, get_youtube_dl_path},
};
use dotenvy_macro::dotenv;

#[derive(Clone, Debug, ValueEnum)]
enum Mode {
    Add,
    Gen,
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

    #[arg(short, long, default_value = "ProTV.txt")]
    output: PathBuf,
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
        Mode::Gen => gen(pool, ytdl, args.output).await,
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

async fn gen(pool: Pool<MySql>, _ytdl: PathBuf, output: PathBuf) -> Result<()> {
    let mut file = File::create(output)?;
    let mut conn = pool.acquire().await?;

    println!("Fetching videos");
    let videos = get_all_videos(&mut conn).await?;

    println!("Generating file");
    for video in videos {
        let channel_handle = video
            .channel_handle
            .map_or(video.channel_id, |handle| handle.chars().skip(1).collect());
        let channel_name = video.channel_name.unwrap_or(channel_handle);
        writeln!(file, "@https://shay.loan/{}", &video.id)?;
        writeln!(file, "#{} {}", video.tags.join(" "), &video.id)?;
        writeln!(file, "{} - {}", channel_name, &video.title)?;
        writeln!(file)?;
    }

    Ok(())
}

async fn old(pool: Pool<MySql>, ytdl: PathBuf, limit: u8) -> Result<()> {
    let mut conn = pool.acquire().await?;

    let channels = get_oldest_channels(&mut conn, limit).await?;
    for channel in channels {
        let channel = channel.handle.unwrap_or(channel.id);
        let _ = get(&mut conn, &ytdl, channel).await;
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

    let Some(single_video_option) = single_videos.first() else {
        bail!("No video found");
    };

    let Some(single_video) = single_video_option else {
        bail!("No video found");
    };

    for single_video_option in &single_videos {
        let Some(single_video) = single_video_option else {
            continue
        };

        let Some(channel_id) = &single_video.channel_id else {
            continue;
        };

        let video = Video {
            id: single_video.id.to_owned(),
            title: single_video.title.to_owned(),
            tags: get_tags(single_video),
            channel_id: channel_id.to_owned(),
            channel_handle: single_video.uploader_id.to_owned(),
            channel_name: None,
        };

        println!("Video: {}", video.title);
        let _ = upsert_video(conn, video).await;
    }

    if let Some(channel_id) = &single_video.channel_id {
        let channel = Channel {
            id: channel_id.to_owned(),
            handle: single_video.uploader_id.to_owned(),
            updated_at: Some(SystemTime::now().into()),
            video_count: single_videos.len() as i32,
        };

        println!("Channel: {}", channel.id);
        let _ = upsert_channel(conn, channel).await;
    }

    Ok(())
}
