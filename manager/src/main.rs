use std::{fs::File, io::Write, path::PathBuf};

use anyhow::{bail, Result};
use clap::{ArgAction, Parser, ValueEnum};
use common::{
    sqlx::{
        get_biggest_channels,
        get_channels,
        get_oldest_channels,
        get_smallest_channels,
        get_tagless_videos,
        get_tags,
        get_videos,
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
use indicatif::ProgressBar;
use manager::Entries::{Channels, Videos};

#[cfg(feature = "read-write")]
const DATABASE_URL: &str = dotenvy_macro::dotenv!("DATABASE_URL");

#[cfg(not(feature = "read-write"))]
const DATABASE_URL: &str = env!("DATABASE_URL");

#[derive(Clone, Debug, PartialEq, ValueEnum)]
enum Mode {
    Add,
    Big,
    Few,
    Gen,
    Old,
    Tag,
}

#[derive(Clone, Debug, Parser)]
#[command()]
struct Args {
    /// Select a mode of operation.
    #[arg(value_enum, short, long)]
    mode: Mode,

    /// Select a channel to fetch. (Add mode only)
    #[arg(short, long)]
    channel: Option<String>,

    /// Limit of channels/videos to fetch. (Big, Few, Old, and Tag modes only)
    #[arg(short, long, default_value_t = 1)]
    limit: u32,

    /// Output file path for ProTV playlist file. (Gen mode only)
    #[arg(short, long, default_value = "ProTV.txt")]
    output: PathBuf,

    /// True = Fetch just the playlist pages, False = Fetch all videos and metadata.
    #[arg(short, long, default_value_t = true, action = ArgAction::Set)]
    flat_playlist: bool,

    /// Name of the playlist to generate from the database. (Example: General or Music)
    #[arg(short, long, default_value = "General")]
    playlist: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let ytdl = get_youtube_dl_path().await?;
    let pool = MySqlPoolOptions::new().connect(DATABASE_URL).await?;

    #[cfg(not(feature = "read-write"))]
    if args.mode != Mode::Gen {
        println!("This manager binary was compiled with read-only database access");
        println!("You may only use Gen mode to generate ProTV playlist files");
        return Ok(());
    }

    match args.mode {
        Mode::Add => add(pool, ytdl, args).await,
        Mode::Big => get(pool, ytdl, args).await,
        Mode::Few => get(pool, ytdl, args).await,
        Mode::Gen => gen(pool, ytdl, args).await,
        Mode::Old => get(pool, ytdl, args).await,
        Mode::Tag => get(pool, ytdl, args).await,
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

    println!("Fetching channels");
    let channels = get_channels(&mut conn, args.playlist).await?;
    let pb = ProgressBar::new(channels.len() as u64);

    println!("Generating file");
    for channel in channels {
        pb.inc(1);

        let videos = get_videos(&mut conn, channel.id).await?;
        for video in videos {
            let channel_name = channel.name.clone().unwrap_or(video.channel_id);
            let mut tags = video.tags.0.unwrap_or_default();
            tags.insert(0, video.id.to_owned());

            writeln!(file, "@https://shay.loan/{}", video.id)?;
            writeln!(file, "#{}", tags.join(" "))?;
            writeln!(file, "{} - {}", channel_name, video.title)?;
            writeln!(file)?;
        }
    }

    pb.finish_with_message("Done");

    Ok(())
}

async fn get(pool: Pool<MySql>, ytdl: PathBuf, args: Args) -> Result<()> {
    let mut conn = pool.acquire().await?;

    let entries = match args.mode {
        Mode::Add => unreachable!(),
        Mode::Big => Channels(get_biggest_channels(&mut conn, args.limit).await?),
        Mode::Few => Channels(get_smallest_channels(&mut conn, args.limit).await?),
        Mode::Gen => unreachable!(),
        Mode::Old => Channels(get_oldest_channels(&mut conn, args.limit).await?),
        Mode::Tag => Videos(get_tagless_videos(&mut conn, args.limit).await?),
    };

    match entries {
        Channels(channels) => {
            for channel in channels {
                let _ = try_update_channel(&mut conn, &ytdl, channel.id, args.flat_playlist).await;
            }
        }
        Videos(videos) => {
            for video in videos {
                let _ = try_update_video(&mut conn, &ytdl, video, args.flat_playlist).await;
            }
        }
    };

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

async fn try_update_video(
    pool: &mut PoolConnection<MySql>,
    ytdl: &PathBuf,
    mut video: Video,
    flat_playlist: bool,
) -> Result<()> {
    let url = format!("https://youtube.com/watch?v={}", video.id);
    let single_video = get_single_video(ytdl, url, flat_playlist)?;

    video.tags = get_tags(single_video.tags, Some(vec![]));

    println!("Video: {}", video.title);
    let _ = upsert_video(pool, video).await;

    Ok(())
}
