extern crate log;
#[macro_use]
extern crate rocket;

use std::{
    collections::HashMap,
    env,
    path::PathBuf,
    time::{Duration, SystemTime},
};

use common::youtube_dl::{download_yt_dlp, proxy_video};
use regex::Regex;
use rocket::{
    fs::NamedFile,
    response::Redirect,
    tokio::{sync::RwLock, time},
    Build,
    Request,
    Rocket,
    State,
};

const EXPIRE_REGEX: &str = r"exp(?:ir(?:es?|ation))?=(\d+)";
const YOUTUBE_REGEX: &str = r#"(?x)^/
    (?:https?://)?
    (?:www\.)?
    (?:
        youtube\.com/watch\?v=|
        youtube\.com/shorts/|
        youtu\.be/
    )?
    ([0-9A-Za-z_-]{11})
    (?:.+)?
$"#;

#[derive(Clone)]
struct CachedVideo {
    exp: SystemTime,
    url: String,
}

struct RocketState {
    cache: RwLock<HashMap<String, Option<CachedVideo>>>,
    expire_regex: Regex,
    youtube_dl_path: PathBuf,
    youtube_regex: Regex,
}

#[get("/")]
async fn index() -> std::io::Result<NamedFile> {
    NamedFile::open("index.html").await
}

#[catch(404)]
async fn proxy(req: &Request<'_>) -> Result<Redirect, &'static str> {
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

        debug!("Attempting to proxy video with yt-dlp");
        let Ok(redirect_url) = proxy_video(&state.youtube_dl_path, &video_url) else {
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
                url: redirect_url.to_owned(),
            }),
        );

        info!("Processed {video_url}, redirecting...");
        return Ok(Redirect::temporary(redirect_url));
    }
}

#[launch]
async fn rocket() -> Rocket<Build> {
    let state = RocketState {
        cache: Default::default(),
        expire_regex: Regex::new(EXPIRE_REGEX).unwrap(),
        youtube_dl_path: download_yt_dlp(env::temp_dir()).await.unwrap(),
        youtube_regex: Regex::new(YOUTUBE_REGEX).unwrap(),
    };

    rocket::build()
        .manage(state)
        .mount("/", routes![index])
        .register("/", catchers![proxy])
}
