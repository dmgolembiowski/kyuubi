#![allow(unused)]
use anyhow::Result;
use dotenv::dotenv;
use pin_project::pin_project;
use qbit_api_rs::{client::QbitClient, error::ClientError};
use serde::{Deserialize, Serialize};
// use std::net::SocketAddr;
use std::{
    os::fd::{BorrowedFd, RawFd},
    sync::Arc,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{unix::SocketAddr, TcpListener, UnixListener, UnixSocket, UnixStream},
    pin,
    sync::{broadcast, Mutex},
    time::Duration,
};
use tracing::{debug, error, info, warn};
pub(crate) mod errors;

pub(crate) mod helper;
use helper::setup;

pub(crate) mod state;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenv().ok();
    let _ = setup().await;

    let client = QbitClient::new_from_env().unwrap();
    let dur = Duration::from_secs(
        std::env::var("QBIT_DURATION")
            .unwrap_or_else(|_| "3600".to_string())
            .parse::<u64>()
            .unwrap(),
    );
    let qb = Arc::new(Mutex::new(Kyuubi::new(client, dur.clone())));
    let rb = Arc::clone(&qb);
    let qbh = tokio::spawn(async move {
        let mut lock = rb.lock().await;
        lock.spawn().await
    });

    let ingress = tokio::spawn(kyuubi_socket_daemon("/tmp/kyuubi.sock"));

    tokio::select! {
        _ = ingress => {},
        _ = qbh => {},
    };

    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct Job {
    pub kind: JobKind,
    pub payload: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub(crate) enum JobKind {
    Ping,
    Pong,
    AddMagnetUrl,
    DownloadTorrentFromUrl,
    AddTorrentFile,
}

async fn kyuubi_socket_daemon(path_to_socket: &str) {
    info!("Spawning the kyuubi socket daemon");
    let _ = tokio::task::spawn_blocking(|| async { tokio::fs::remove_file(&path_to_socket).await });
    let listener = UnixListener::bind(&path_to_socket).unwrap();
    let (tx, rx) = broadcast::channel::<(Job, BorrowedFd)>(10);
    loop {
        let (mut socket, client_addr) = listener.accept().await.unwrap();

        info!("Incoming connection from {client_addr:?}");

        let relay = tx.clone();
        let mut relay_rx = relay.subscribe();
        tokio::spawn(async move {
            let (reader, mut writer) = socket.split();
            let peer_addr: SocketAddr = reader.peer_addr().unwrap();
            let fileno = writer.peer_addr().unwrap()
            let mut buf_reader = BufReader::new(reader);
            let mut buffer: Vec<u8> = Vec::with_capacity(4096);
            loop {
                info!("Entered the working-portion loop on the socket daemon");

                tokio::select! {
                    _ = buf_reader.read_until(b'\n', &mut buffer) => {
                        let job: Job = serde_json::from_slice(&buffer[..]).unwrap();
                        let fileno = socket.write_u16_leget_file_descriptor();
                        
                        buffer.clear();
                        },
                    recv = relay_rx.recv() => {
                        let data = recv.unwrap();
                        let (job, client_addr): (Job, std::net::SocketAddr) = data;

                        // TODO: Act on the job here

                        if &client_addr.as_pathname() == &socket.local_addr().expect("..").as_pathname() {
                            debug!("We sent a message to ourselves: {:?}", &job);
                        }
                        else {
                            debug!("Echoing request to {client_addr:?}");
                            writer.write_all(b"\r\n").await.unwrap();
                        }

                    },
                }
            }
        });
    }
}

/// Programmatic interface to an auto-refreshing
/// Qbittorrent API session. According to its 4.1
/// specification, client sessions expire after 3600
/// seconds (or 10 minutes).
///
/// To enable long-term interactivity with the process,
/// [`Kyuubi`]() will automatically log back in whenever
/// it expires.
#[pin_project]
pub struct Kyuubi {
    #[pin]
    client: QbitClient,
    duration: Duration,
}

impl Kyuubi {
    /// Creates a new `Kyuubi` instance.
    pub fn new(client: QbitClient, duration: Duration) -> Self {
        Self { client, duration }
    }

    #[allow(unreachable_code)]
    pub async fn spawn(&mut self) {
        // First establish a connection using self.client.auth_login
        // and if it succeeds return a Future that loops every `QBIT_DURATION`
        // to fix its client session handle.
        // use tokio::macros::support::poll_fn;
        debug!("Attempting initial login to Qbittorrent web server");
        let _ = &mut self.auth_login().await.map_err(|e| {
            error!("Failed initial connection {e:?}");
            return;
        });

        tokio::pin! {
            let interval = tokio::time::interval(self.duration);
        }
        debug!("Entering auto-refreshing loop for the QbitClient");
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    info!("Preparing to authenaticate....");
                    let _ = self.auth_login().await.map_err(|e| {
                        error!("Failed connection {e:?}");
                    });
                }
            };
        }
    }
    pub async fn auth_login(&mut self) -> Result<(), ClientError> {
        debug!("Attempting login.");
        match self.client.auth_login().await {
            Ok(_) => {
                let ver = &self.client.app_version().await?;
                info!("Login successful. Using API version {ver:?}.")
            }
            Err(e) => {
                error!("Failed login {e:?}");
                return Err(e.into());
            }
        }
        Ok(())
    }
}
