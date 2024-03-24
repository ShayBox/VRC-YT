use std::time::{Duration, SystemTime};

#[cfg(feature = "database")]
use common::sqlx::{insert_channel, upsert_video, Channel, Video};
use common::youtube_dl::{get_format_url, get_single_video};
use rocket::{response::Redirect, tokio::time, Request, State};
#[cfg(feature = "database")]
use rocket_db_pools::Connection;

#[cfg(feature = "database")]
use crate::VRChatYouTube;
use crate::{CachedVideo, RocketState};

#[catch(404)]
pub async fn proxy(req: &Request<'_>) -> Result<Redirect, &'static str> {
    #[cfg(feature = "database")]
    let mut conn = req.guard::<Connection<VRChatYouTube>>().await.unwrap();
    let state = req.guard::<&'_ State<RocketState>>().await.unwrap();
    let request_uri = req.uri().to_string();

    debug!("Attempting to capture video id from request uri with regex");
    let Some(captures) = state.youtube_regex.captures(&request_uri) else {
        return Err("Unable to capture video id from request uri with regex");
    };

    debug!("Attempting to get video id from capture");
    let Some(video_id) = captures.get(1).map(|m| m.as_str()) else {
        return Err("Unable to get video id from capture");
    };

    let video_url = format!("https://youtu.be/{video_id}");
    info!("Processing {video_url}...");

    loop {
        debug!("Checking if {video_id} is in the cache");
        if let Some(cached_video) = {
            let cache = state.cache.read().await;
            cache.get(video_id).cloned()
        } {
            debug!("Checking if {video_id} is fully cached");
            if let Some(cached_video) = cached_video {
                debug!("Checking if {video_id} is expired");
                if cached_video.exp > SystemTime::now() {
                    info!("Processed {video_url}, redirecting...");
                    return Ok(Redirect::temporary(cached_video.url));
                }

                info!("{video_id} is expired, removing...");
                state.cache.write().await.remove(video_id);
            }

            info!("{video_id} is being cached, waiting...");
            time::sleep(Duration::from_millis(500)).await;
            continue;
        }

        info!("{video_id} is not cached, caching...");
        state.cache.write().await.insert(video_id.to_string(), None);

        debug!("Attempting to get single video with yt-dlp");
        let Ok(single_video) = get_single_video(&state.youtube_dl_path, &video_url, true) else {
            state.cache.write().await.remove(video_id);
            return Err("Unable to proxy video with yt-dlp");
        };

        debug!("Attempting to get format url with yt-dlp");
        let Ok(redirect_url) = get_format_url(&single_video) else {
            state.cache.write().await.remove(video_id);
            return Err("Unable to proxy video with yt-dlp");
        };

        debug!("Attempting to capture expiration from redirect url with regex");
        let mut exp = SystemTime::now() + Duration::from_secs(600);
        if let Some(captures) = state.expire_regex.captures(&redirect_url) {
            debug!("Attempting to get expiration from capture");
            if let Some(expiration) = captures.get(1) {
                debug!("Attempting to parse expiration into seconds");
                if let Ok(secs) = expiration.as_str().parse::<u64>() {
                    debug!("Captured and parsed expiration {secs}");
                    exp = SystemTime::UNIX_EPOCH + Duration::from_secs(secs);
                }
            }
        }

        debug!("Updating cache with redirect url");
        state.cache.write().await.insert(
            video_id.to_string(),
            Some(CachedVideo {
                exp,
                url: redirect_url.clone(),
            }),
        );

        #[cfg(feature = "database")]
        {
            // common SQLx version must match rocket_db_pools SQLx version
            if let Ok(channel) = Channel::try_from(*single_video.clone()) {
                if let Err(error) = insert_channel(&mut conn, channel).await {
                    eprintln!("Error inserting channel: {error}");
                }
            };
            if let Ok(video) = Video::try_from(*single_video.clone()) {
                if let Err(error) = upsert_video(&mut conn, video).await {
                    eprintln!("Error upserting video: {error}");
                }
            }
        }

        info!("Processed {video_url}, redirecting...");
        return Ok(Redirect::temporary(redirect_url));
    }
}
