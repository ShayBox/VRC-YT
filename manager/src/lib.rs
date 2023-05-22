use std::future::Future;

use common::sqlx::{Channel, Error, MySql, PoolConnection, Video};

pub enum Entries {
    Channels(Vec<Channel>),
    Videos(Vec<Video>),
}

#[allow(clippy::type_complexity)]
pub enum EntriesFn {
    ChannelsFn(
        fn(&mut PoolConnection<MySql>, u32) -> dyn Future<Output = Result<Vec<Channel>, Error>>,
    ),
    VideosFn(fn(&mut PoolConnection<MySql>, u32) -> dyn Future<Output = Result<Vec<Video>, Error>>),
}
