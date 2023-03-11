use std::{
    collections::HashMap,
    env,
    path::PathBuf,
    time::{Duration, SystemTime},
};

use actix_web::{
    get,
    head,
    web::{Data, Redirect},
    App,
    HttpRequest,
    HttpResponse,
    HttpServer,
    Responder,
    Result,
};
use regex::Regex;
use tokio::{sync::RwLock, time};
use tracing::{debug, info};
use vrc_yt_proxy::proxy_video;
use youtube_dl::download_yt_dlp;

const EXP_REGEX: &str = r"exp(?:ir(?:es?|ation))?=(\d+)";
const URL_REGEX: &str = r#"(?x)^/
    (?:https?://)?
    (?:www\.)?
    (?:
        youtube\.com/watch\?v=|
        youtu\.be/
    )?
    ([0-9A-Za-z_-]{11})
    (.+)?
$"#;

#[derive(Clone)]
struct CachedVideo {
    exp: SystemTime,
    url: String,
}

struct AppState {
    cache: RwLock<HashMap<String, Option<CachedVideo>>>,
    exp_regex: Regex,
    url_regex: Regex,
    yt_dlp_path: PathBuf,
}

#[get("/")]
async fn get_index() -> impl Responder {
    use actix_files::NamedFile;

    NamedFile::open("index.html")
}

#[get("/{path}*")]
async fn get_video(req: HttpRequest, state: Data<AppState>) -> Result<impl Responder> {
    video(req, state).await
}

#[head("/{path}*")]
async fn head_video(req: HttpRequest, state: Data<AppState>) -> Result<impl Responder> {
    video(req, state).await
}

async fn video(req: HttpRequest, state: Data<AppState>) -> Result<impl Responder> {
    let video_id = &req.uri().to_string();

    debug!("Attempting to parse {video_id} into capture groups");
    let Some(captures) = state.url_regex.captures(video_id) else {
        let response = HttpResponse::BadRequest().body("Failed to capture regex");
        return Ok(response);
    };

    debug!("Attempting to shadow the video_id with capture group one");
    let Some(video_id) = captures.get(1).map(|m| m.as_str()) else {
        let response = HttpResponse::BadRequest().body("Failed to match regex");
        return Ok(response);
    };

    info!("Processing https://youtu.be/{video_id}");
    loop {
        debug!("Checking if {video_id} is in the cache");
        if let Some(option_video) = {
            let cache = state.cache.read().await;
            cache.get(video_id).cloned()
        } {
            debug!("{video_id} is in the cache, checking if it's fully cached");
            if let Some(cached_video) = option_video {
                debug!("{video_id} is fully cached, checking if it's expired");
                if cached_video.exp > SystemTime::now() {
                    let url = cached_video.url.to_owned();
                    let redirect = Redirect::to(url)
                        .temporary()
                        .respond_to(&req)
                        .map_into_boxed_body();

                    info!("{video_id} was cached");
                    return Ok(redirect);
                } else {
                    debug!("{video_id} is expired, removing and retrying");
                    state.cache.write().await.remove(video_id);
                    continue;
                }
            } else {
                debug!("{video_id} is not fully cached, waiting");
                time::sleep(Duration::from_secs(500)).await;
                continue;
            }
        }

        debug!("{video_id} is not in the cache, partially adding to the cache");
        state.cache.write().await.insert(video_id.to_string(), None);

        debug!("Attempting to proxy {video_id} using yt-dlp");
        let video_url = match proxy_video(video_id.to_string(), &state.yt_dlp_path) {
            Ok(url) => url,
            Err(error) => {
                debug!("{video_id} failed to proxy, removing from the cache");
                state.cache.write().await.remove(video_id);
                let response = HttpResponse::BadRequest().body(error.to_string());
                return Ok(response);
            }
        };

        debug!("Attempting to parse expiration from the video_url");
        let mut exp = SystemTime::now() + Duration::from_secs(600);
        if let Some(exp_captures) = state.exp_regex.captures(&video_url) {
            if let Some(exp_match) = exp_captures.get(1) {
                if let Ok(secs) = exp_match.as_str().parse::<u64>() {
                    debug!("Found expiration {secs}");
                    exp = SystemTime::UNIX_EPOCH + Duration::from_secs(secs);
                }
            }
        };

        state.cache.write().await.insert(
            video_id.to_string(),
            Some(CachedVideo {
                exp,
                url: video_url.to_owned(),
            }),
        );

        let redirect = Redirect::to(video_url)
            .temporary()
            .respond_to(&req)
            .map_into_boxed_body();

        info!("{video_id} is now cached");
        return Ok(redirect);
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();

    let state = Data::new(AppState {
        cache: Default::default(),
        exp_regex: Regex::new(EXP_REGEX).unwrap(),
        url_regex: Regex::new(URL_REGEX).unwrap(),
        yt_dlp_path: download_yt_dlp(env::temp_dir()).await.unwrap(),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .service(get_index)
            .service(get_video)
    })
    .bind((
        env::var("ADDR").unwrap_or("127.0.0.1".into()),
        env::var("PORT").unwrap_or("8000".into()).parse().unwrap(),
    ))?
    .run()
    .await
}
