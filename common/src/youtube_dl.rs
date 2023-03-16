use std::{
    env,
    path::{Path, PathBuf},
};

use thiserror::Error;
use which::which;
use youtube_dl::{download_yt_dlp, Error, SingleVideo, YoutubeDl, YoutubeDlOutput};

#[derive(Debug, Error)]
pub enum YoutubeError {
    #[error("{0}")]
    YoutubeDL(#[from] youtube_dl::Error),

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

pub fn get_output<P, U>(youtube_dl_path: P, url: U) -> Result<YoutubeDlOutput, YoutubeError>
where
    P: AsRef<Path>,
    U: Into<String>,
{
    YoutubeDl::new(url)
        .format("best*[vcodec!=none][acodec!=none]")
        .youtube_dl_path(youtube_dl_path)
        .socket_timeout("15")
        .run()
        .map_err(YoutubeError::YoutubeDL)
}

pub fn get_single_video<P, U>(youtube_dl_path: P, url: U) -> Result<Box<SingleVideo>, YoutubeError>
where
    P: AsRef<Path>,
    U: Into<String>,
{
    let output = get_output(youtube_dl_path, url)?;

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
        if let Some(format_string) = &format.format {
            format_string == video_format_string
        } else {
            false
        }
    }) else {
        return Err(YoutubeError::VideoFormat);
    };

    let Some(video_url) = &video_format.url else {
        return Err(YoutubeError::VideoUrl);
    };

    Ok(video_url.to_owned())
}

pub fn get_tags(single_video: &SingleVideo) -> Vec<String> {
    single_video
        .tags
        .clone()
        .map(|opt_tags| opt_tags.into_iter().flatten().collect())
        .unwrap_or_else(Vec::new)
}
