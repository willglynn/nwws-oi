use crate::*;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

/// A stream of events from NWWS-OI.
///
/// `Stream` automatically re-connects if it was disconnected and generally retries on failure.
pub struct Stream {
    rx: tokio::sync::mpsc::Receiver<StreamEvent>,
}

impl Stream {
    pub fn new<C: Into<Config>>(config: C) -> Self {
        let config = config.into();
        let (tx, rx) = tokio::sync::mpsc::channel(32);

        std::thread::spawn(move || {
            let local = tokio::task::LocalSet::new();

            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            local.spawn_local(run(config, tx));

            rt.block_on(local);
        });

        Self { rx }
    }
}

impl futures::Stream for Stream {
    type Item = StreamEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.rx).poll_recv(cx)
    }
}

async fn run(
    config: Config,
    tx: tokio::sync::mpsc::Sender<StreamEvent>,
) -> Result<(), tokio::sync::mpsc::error::SendError<StreamEvent>> {
    loop {
        tx.send(StreamEvent::ConnectionState(ConnectionState::Connecting))
            .await?;
        run_once(config.clone(), tx.clone()).await?;

        // Ensure a minimum delay
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn run_once(
    config: Config,
    tx: tokio::sync::mpsc::Sender<StreamEvent>,
) -> Result<(), tokio::sync::mpsc::error::SendError<StreamEvent>> {
    let mut conn =
        match tokio::time::timeout(Duration::from_secs(75), Connection::new(config)).await {
            Ok(Ok(conn)) => {
                tx.send(StreamEvent::ConnectionState(ConnectionState::Connected))
                    .await?;
                conn
            }
            Ok(Err(e)) => {
                // Connecting failed
                // Wait a little while or an extra long time before retrying, depending on the cause
                let duration = match e {
                    Error::Configuration(_) | Error::Credentials(_) => 300,
                    _ => 10,
                };

                // Send the error and the disconnect event
                tx.send(StreamEvent::Error(e)).await?;
                tx.send(StreamEvent::ConnectionState(ConnectionState::Disconnected))
                    .await?;

                // Wait
                tokio::time::sleep(Duration::from_secs(duration)).await;

                return Ok(());
            }
            Err(_) => {
                // Connection timed out
                tx.send(StreamEvent::ConnectionState(ConnectionState::Disconnected))
                    .await?;

                return Ok(());
            }
        };

    loop {
        match conn.next_message().await {
            Ok(msg) => tx.send(StreamEvent::Message(msg)).await?,
            Err(e) => {
                tx.send(StreamEvent::Error(e)).await?;
                tx.send(StreamEvent::ConnectionState(ConnectionState::Disconnected))
                    .await?;
                tokio::task::spawn_local(conn.end());

                return Ok(());
            }
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ConnectionState {
    Connecting,
    Connected,
    Disconnected,
}

#[derive(Debug)]
pub enum StreamEvent {
    ConnectionState(ConnectionState),
    Error(Error),
    Message(Message),
}
