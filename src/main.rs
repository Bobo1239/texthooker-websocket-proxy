use anyhow::Result;
use futures_util::{FutureExt, SinkExt, StreamExt};
use log::*;
use tokio::{
    net::TcpListener,
    sync,
    time::{self, Duration},
};
use tokio_tungstenite::tungstenite::{Bytes, Message};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );
    let (tx, _) = sync::broadcast::channel(10);

    let bind_addr = "127.0.0.1:6678";
    let downstream_addresses = [
        // textractor-websocket
        "ws://localhost:6677",
        // mpv_websocket (configured)
        "ws://localhost:6676",
        // agent
        "ws://localhost:9001",
    ];

    let server = TcpListener::bind(bind_addr).await?;
    info!("Listening on {bind_addr}");

    for addr in downstream_addresses {
        let tx = tx.clone();
        tokio::spawn(async move {
            loop {
                if let Ok((socket, _)) = tokio_tungstenite::connect_async(addr).await {
                    info!("Outgoing connection to {addr}");
                    socket
                        .for_each(|msg| async {
                            if let Ok(msg) = msg {
                                // May fail if no there are subscribers which is fine.
                                tx.send(msg).ok();
                            }
                        })
                        .await;
                    info!("Outgoing connection closed to {addr}");
                }

                time::sleep(Duration::from_secs(1)).await;
            }
        });
    }

    while let Ok((stream, peer_addr)) = server.accept().await {
        let mut rx = tx.subscribe();
        tokio::spawn(async move {
            info!("Incoming connection from {peer_addr}");
            let mut websocket = tokio_tungstenite::accept_async(stream).await.unwrap();
            let mut keepalive_interval = time::interval(Duration::from_secs(30));
            loop {
                let msg = futures_util::select! {
                    _ = keepalive_interval.tick().fuse() => {
                        // Ensure that we don't let the WebSocket connection get timed out by
                        // sending a periodic ping
                        Some(Message::Ping(Bytes::new()))
                    }
                    msg = rx.recv().fuse() => {
                        Some(msg.unwrap())
                    }
                    msg = websocket.next() => {
                        if msg.is_none() {
                            // Socket was closed
                            break;
                        }
                        // We're also reading messages so pongs don't fill up our input queue
                        None
                    }
                };
                if let Some(msg) = msg {
                    if let Err(e) = websocket.send(msg).await {
                        error!("Send failed: {:?}", e);
                    }
                }
            }
            info!("Incoming connection closed from {peer_addr}");
        });
    }

    Ok(())
}
