use std::path::Path;

use thiserror::Error;
pub use youtube_dl::{download_yt_dlp, YoutubeDl};
use youtube_dl::{SingleVideo, YoutubeDlOutput};

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

pub fn proxy_video<P, U>(youtube_dl_path: P, url: U) -> Result<String, YoutubeError>
where
    P: AsRef<Path>,
    U: Into<String>,
{
    let single_video = get_single_video(youtube_dl_path, url)?;

    let Some(video_formats) = single_video.formats else {
        return Err(YoutubeError::VideoFormats);
    };

    let Some(video_format_string) = single_video.format else {
        return Err(YoutubeError::VideoFormatString);
    };

    let Some(video_format) = video_formats.iter().find(|format| {
        if let Some(format_string) = &format.format {
            format_string == &video_format_string
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
