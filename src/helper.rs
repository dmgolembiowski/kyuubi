#![allow(unused)]
use console_subscriber::ConsoleLayer;
use futures::{
    channel::mpsc::{channel, Receiver},
    SinkExt, /* StreamExt, TryStreamExt,*/
};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::Arc;
use std::{error::Error, time::SystemTime};
use time::macros::offset;
use tokio::sync::{mpsc, Mutex};
use tokio::time::Instant;
use tokio_stream::StreamExt;
use tracing::{self, debug, error, info, instrument, warn};
use tracing_rolling::{Checker, Daily};
use tracing_subscriber::{self as ts, prelude::*, EnvFilter};

pub(crate) async fn setup() -> Result<(), Box<dyn Error>> {
    // TODO:
    // Replace this with a getter/setter based on whether
    // this is run from docker
    let log_dir = std::env::var("LOGS_NEW_DEV").unwrap();

    let console_layer = ConsoleLayer::builder().with_default_env().spawn();

    let console_fmt_layer = ts::fmt::layer()
        .with_ansi(atty::is(atty::Stream::Stdout))
        .with_target(true);

    /*
    let log_fmt_layer = ts::fmt::layer()
        .with_ansi(false)
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .with_writer(|| {
            Daily::new(log_dir, "[year][month][day]", offset!(+8))
                .buffered()
                .build()
                .unwrap()
        });
    */
    let filter_layer =
        EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("debug"))?;

    let registry = ts::registry()
        // .with(log_fmt_layer)
        .with(console_layer)
        .with(console_fmt_layer)
        .with(filter_layer)
        .init();

    Ok(())
}

pub(crate) type Observed = notify::Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)>;

pub(crate) fn async_watcher() -> Observed {
    let (mut tx, rx) = channel(1);
    let watcher = RecommendedWatcher::new(
        move |res| {
            futures::executor::block_on(async {
                tx.send(res).await.unwrap();
            })
        },
        <_>::default(),
    )?;
    Ok((watcher, rx))
}

pub(crate) async fn aionotify<P: AsRef<Path>>(
    inotify_path: P,
    db: Option<Arc<Mutex<&'static mut sled::Db>>>,
) -> notify::Result<()> {
    let (mut notifier, mut rx) = async_watcher()?;
    let without_db = !db.clone().is_some();

    notifier.watch(inotify_path.as_ref(), RecursiveMode::Recursive)?;

    if without_db {
        while let Some(res) = rx.next().await {
            match res {
                Ok(event) => {
                    warn!("Inotify event: {:?}", event);
                }
                Err(e) => error!("Inotify ERROR: {e:?}"),
            }
        }
    } else {
        // let mut db = db.expect("the db").lock().await;
        let fs_changes = db
            .unwrap()
            .lock()
            .await
            .open_tree(b"fs_changes")
            .map_err(|e| notify::Error::generic(&e.to_string()))?;
        while let Some(res) = rx.next().await {
            match res {
                Ok(event) => {
                    warn!("Inotify event: {:?}", &event);
                    let it = serde_json::to_vec(&event).unwrap();
                    let now = SystemTime::now()
                        .duration_since(std::time::SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                        .to_string();
                    let src = format!("{:p}", &event);
                    let key = format!("{now:}_{src:}");
                    fs_changes
                        .insert(key.as_bytes(), it)
                        .map_err(|e| notify::Error::generic(&e.to_string()))?;
                    /*
                    while let path = futures::stream::iter(event.paths.iter()) {
                        debug!("Inotify event at path: {:?}", &path);
                        let it = serde_json::to_vec(&event).unwrap();
                        let now = SystemTime::now()
                            .duration_since(std::time::SystemTime::UNIX_EPOCH)
                            .unwrap()
                            .as_secs()
                            .to_string();
                        let src = format!("{:p}", &event);
                        let key = format!("{now:}_{src:}");
                        fs_changes.insert(key, it).expect("compile time errors");
                    }*/
                }
                Err(e) => {
                    error!("Inotify ERROR: {e:?}");
                }
            }
        }
    }

    Ok(())
}
