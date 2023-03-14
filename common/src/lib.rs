use std::{
    fmt::{Display, Formatter},
    path::PathBuf,
};

pub use youtube_dl::{YoutubeDl, YoutubeDlOutput::SingleVideo, download_yt_dlp};

#[derive(Debug)]
pub enum Error {
    YoutubeDL(youtube_dl::Error),
    SingleVideo,
    FormatsNotFound,
    UrlNotFound,
}

impl std::error::Error for Error {}
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::YoutubeDL(error) => write!(f, "{}", error),
            Error::SingleVideo => write!(f, "Single videos only supported"),
            Error::FormatsNotFound => write!(f, "Video formats were not found"),
            Error::UrlNotFound => write!(f, "Video URL was not found"),
        }
    }
}

pub fn proxy_video(url: impl Into<String>, yt_dlp_path: &PathBuf) -> Result<String, Error> {
    let output = YoutubeDl::new(url)
        .format("best*[vcodec!=none][acodec!=none]")
        .youtube_dl_path(yt_dlp_path)
        .socket_timeout("15")
        .run()
        .map_err(Error::YoutubeDL)?;

    let SingleVideo(single_video) = output else {
        return Err(Error::SingleVideo);
    };

    let Some(video_formats) = single_video.formats else {
        return Err(Error::FormatsNotFound);
    };

    let Some(video_format_string) = single_video.format else {
        return Err(Error::FormatsNotFound);
    };

    let Some(video_format) = video_formats.iter().find(|format| {
        if let Some(format_string) = &format.format {
            format_string == &video_format_string
        } else {
            false
        }
    }) else {
        return Err(Error::FormatsNotFound);
    };

    let Some(video_url) = &video_format.url else {
        return Err(Error::UrlNotFound);
    };

    Ok(video_url.to_owned())
}
