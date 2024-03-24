use std::{
    env,
    path::{Path, PathBuf},
};

use thiserror::Error;
use which::which;
use youtube_dl::{download_yt_dlp, Error, Playlist, SingleVideo, YoutubeDl, YoutubeDlOutput};

#[derive(Debug, Error)]
pub enum YoutubeError {
    #[error("{0}")]
    YoutubeDL(#[from] youtube_dl::Error),

    #[error("Playlists only")]
    Playlist,

    #[error("Single videos only")]
    SingleVideo,

    #[error("Unable to find video formats")]
    VideoFormats,

    #[error("Unable to find video format string")]
    VideoFormatString,

    #[error("Unable to find video format")]
    VideoFormat,

    #[error("Unable to find video url")]
    VideoUrl,
}

pub async fn get_youtube_dl_path() -> Result<PathBuf, Error> {
    match which("yt-dlp") {
        Ok(path) => Ok(path),
        Err(_) => Ok(download_yt_dlp(env::temp_dir()).await?),
    }
}

pub fn get_output<P, U>(
    youtube_dl_path: P,
    url: U,
    flat_playlist: bool,
) -> Result<YoutubeDlOutput, YoutubeError>
where
    P: AsRef<Path>,
    U: Into<String>,
{
    // https://blog.natalie.ee/posts/building-dynamic-vrchat-world/#how-vrchat-media-players-work-and-a-little-optimization
    YoutubeDl::new(url)
        .flat_playlist(flat_playlist)
        .format("mp4[height>=?64][width>=?64]/best[height>=?64][width>=?64]")
        .ignore_errors(true)
        .socket_timeout("15")
        .youtube_dl_path(youtube_dl_path)
        .run()
        .map_err(YoutubeError::YoutubeDL)
}

pub fn get_playlist<P, U>(
    youtube_dl_path: P,
    url: U,
    flat_playlist: bool,
) -> Result<Box<Playlist>, YoutubeError>
where
    P: AsRef<Path>,
    U: Into<String>,
{
    let output = get_output(youtube_dl_path, url, flat_playlist)?;

    let YoutubeDlOutput::Playlist(playlist) = output else {
        return Err(YoutubeError::Playlist);
    };

    Ok(playlist)
}

pub fn get_single_video<P, U>(
    youtube_dl_path: P,
    url: U,
    flat_playlist: bool,
) -> Result<Box<SingleVideo>, YoutubeError>
where
    P: AsRef<Path>,
    U: Into<String>,
{
    let output = get_output(youtube_dl_path, url, flat_playlist)?;

    let YoutubeDlOutput::SingleVideo(single_video) = output else {
        return Err(YoutubeError::SingleVideo);
    };

    Ok(single_video)
}

pub fn get_format_url(single_video: &SingleVideo) -> Result<String, YoutubeError> {
    let Some(ref video_formats) = single_video.formats else {
        return Err(YoutubeError::VideoFormats);
    };

    let Some(ref video_format_string) = single_video.format else {
        return Err(YoutubeError::VideoFormatString);
    };

    let Some(video_format) = video_formats.iter().find(|format| {
        format
            .format
            .as_ref()
            .map_or(false, |format_string| format_string == video_format_string)
    }) else {
        return Err(YoutubeError::VideoFormat);
    };

    let Some(video_url) = &video_format.url else {
        return Err(YoutubeError::VideoUrl);
    };

    Ok(video_url.to_owned())
}
