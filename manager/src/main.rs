use std::{cmp::Ordering, fs::File, io::Write, path::PathBuf};

use anyhow::{bail, Result};
use clap::{ArgAction, Parser, ValueEnum};
use common::{
    sqlx::{
        get_all_videos,
        get_oldest_channels,
        get_smallest_channels,
        get_tagless_videos,
        get_tags,
        upsert_channel,
        upsert_video,
        Channel,
        MySql,
        MySqlPoolOptions,
        PlaylistWrapper,
        Pool,
        PoolConnection,
        Video,
    },
    youtube_dl::{get_playlist, get_single_video, get_youtube_dl_path},
};
use dotenvy_macro::dotenv;

#[derive(Clone, Debug, ValueEnum)]
enum Mode {
    Add,
    Big,
    Gen,
    Old,
    Tag,
}

#[derive(Clone, Debug, Parser)]
#[command()]
struct Args {
    #[arg(value_enum, short, long)]
    mode: Mode,

    #[arg(short, long)]
    channel: Option<String>,

    #[arg(short, long, default_value_t = 1)]
    limit: u32,

    #[arg(short, long, default_value = "ProTV.txt")]
    output: PathBuf,

    #[arg(short, long, default_value_t = true, action = ArgAction::Set)]
    flat_playlist: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let ytdl = get_youtube_dl_path().await?;
    let pool = MySqlPoolOptions::new()
        .connect(dotenv!("DATABASE_URL"))
        .await?;

    match args.mode {
        Mode::Add => add(pool, ytdl, args).await,
        Mode::Big => chn(pool, ytdl, args).await,
        Mode::Gen => gen(pool, ytdl, args).await,
        Mode::Old => chn(pool, ytdl, args).await,
        Mode::Tag => vid(pool, ytdl, args).await,
    }
}

async fn add(pool: Pool<MySql>, ytdl: PathBuf, args: Args) -> Result<()> {
    let mut conn = pool.acquire().await?;
    let Some(channel) = args.channel else {
        bail!("Channel required");
    };

    try_update_channel(&mut conn, &ytdl, channel, args.flat_playlist).await?;

    Ok(())
}

async fn gen(pool: Pool<MySql>, _ytdl: PathBuf, args: Args) -> Result<()> {
    let mut file = File::create(args.output)?;
    let mut conn = pool.acquire().await?;

    println!("Fetching videos");
    let mut videos = get_all_videos(&mut conn).await?;

    println!("Sorting videos");
    videos.sort_by(|a, b| match a.channel_name.cmp(&b.channel_name) {
        Ordering::Equal => a.title.cmp(&b.title),
        ordering => ordering,
    });

    println!("Generating file");
    for video in videos {
        let channel_name = video.channel_name.unwrap_or(video.channel_id);
        writeln!(file, "@https://shay.loan/{}", &video.id)?;
        if let Some(tags) = video.tags.0 {
            writeln!(file, "#{} {}", tags.join(" "), &video.id)?;
        }
        writeln!(file, "{} - {}", channel_name, &video.title)?;
        writeln!(file)?;
    }

    Ok(())
}

async fn chn(pool: Pool<MySql>, ytdl: PathBuf, args: Args) -> Result<()> {
    let mut conn = pool.acquire().await?;

    let channels = match args.mode {
        Mode::Add => vec![],
        Mode::Big => get_smallest_channels(&mut conn, args.limit).await?,
        Mode::Gen => vec![],
        Mode::Old => get_oldest_channels(&mut conn, args.limit).await?,
        Mode::Tag => vec![],
    };

    for channel in channels {
        match try_update_channel(&mut conn, &ytdl, channel.id, args.flat_playlist).await {
            Ok(_) => (),
            Err(error) => println!("{error}"),
        }
    }

    Ok(())
}

async fn vid(pool: Pool<MySql>, ytdl: PathBuf, args: Args) -> Result<()> {
    let mut conn = pool.acquire().await?;

    let videos = match args.mode {
        Mode::Add => vec![],
        Mode::Big => vec![],
        Mode::Gen => vec![],
        Mode::Old => vec![],
        Mode::Tag => get_tagless_videos(&mut conn, args.limit).await?,
    };

    for mut video in videos {
        let url = format!("https://youtube.com/watch?v={}", video.id);
        let single_video = get_single_video(&ytdl, url, args.flat_playlist)?;

        video.tags = get_tags(single_video.tags, Some(vec![]));

        println!("Video: {}", video.title);
        let _ = upsert_video(&mut conn, video).await;
    }

    Ok(())
}

async fn try_update_channel(
    pool: &mut PoolConnection<MySql>,
    ytdl: &PathBuf,
    channel: String,
    flat_playlist: bool,
) -> Result<()> {
    let url = format!("https://youtube.com/channel/{channel}/videos");
    println!("Fetching {url}");

    let playlist = get_playlist(ytdl, &url, flat_playlist)?;
    let Ok(channel) = Channel::try_from(*playlist.to_owned()) else {
        bail!("Channel not found");
    };

    let videos: Vec<Video> = PlaylistWrapper::from(*playlist).into();
    for video in videos {
        println!("Video: {}", video.title);
        let _ = upsert_video(pool, video).await;
    }

    upsert_channel(pool, channel).await?;

    Ok(())
}
