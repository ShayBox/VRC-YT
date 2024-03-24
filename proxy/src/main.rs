#![allow(clippy::option_if_let_else)]

mod route;

#[macro_use]
extern crate rocket;

use std::{collections::HashMap, path::PathBuf, time::SystemTime};

use common::youtube_dl::get_youtube_dl_path;
use regex::Regex;
use rocket::tokio::sync::RwLock;
#[cfg(feature = "database")]
use rocket_db_pools::{
    sqlx::{self},
    Database,
};

use crate::route::prelude::*;

const EXPIRE_REGEX: &str = r"exp(?:ir(?:es?|ation))?=(\d+)";
const YOUTUBE_REGEX: &str = r"(?x)^/
    (?:https?://)?
    (?:www\.)?
    (?:
        youtube\.com/watch\?v=|
        youtube\.com/shorts/|
        youtu\.be/
    )?
    ([0-9A-Za-z_-]{11})
    (?:.+)?
$";

#[cfg(feature = "database")]
#[derive(Database)]
#[database("VRC_YT")]
struct VRChatYouTube(sqlx::MySqlPool);

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

#[launch]
async fn rocket() -> _ {
    #[cfg(debug_assertions)]
    dotenvy::dotenv().expect(".env file not found");

    let state = RocketState {
        cache: RwLock::default(),
        expire_regex: Regex::new(EXPIRE_REGEX).unwrap(),
        youtube_dl_path: get_youtube_dl_path().await.unwrap(),
        youtube_regex: Regex::new(YOUTUBE_REGEX).unwrap(),
    };

    #[allow(unused_mut)]
    let mut rocket = rocket::build()
        .manage(state)
        .mount("/", routes![root])
        .register("/", catchers![proxy]);

    #[cfg(feature = "database")]
    {
        rocket = rocket.attach(VRChatYouTube::init());
    }

    rocket
}
