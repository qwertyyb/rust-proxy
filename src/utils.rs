use std::io::ErrorKind;
use std::sync::Arc;

use log::{debug, error};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::task;

pub async fn pipe(current: Arc<Mutex<TcpStream>>, next: Arc<Mutex<TcpStream>>) {
    loop {
        let mut buf = [0; 10240];
        let mut current = current.lock().await;
        let mut next = next.lock().await;
        debug!("pip: {:?} -> {:?}", current.peer_addr(), next.peer_addr());
        match current.read(&mut buf).await {
            Ok(0) => {
                next.shutdown().await.unwrap();
                debug!("pipe ok");
                break;
            }
            Ok(size) => {
                next.write(&buf[..size]).await.unwrap();
            }
            Err(err) => {
                if err.kind() == ErrorKind::WouldBlock {
                    continue;
                } else {
                    debug!("Error: {err}, kind: {}", err.kind());
                    break;
                }
            }
        }
    }
}

pub async fn exchange(mut client: TcpStream, mut server: TcpStream) {
    debug!("start exchange");
    task::spawn(async move {
        let (mut read1, mut write1) = client.split();
        let (mut read2, mut write2) = server.split();

        let copy1 = tokio::io::copy(&mut read1, &mut write2);
        let copy2 = tokio::io::copy(&mut read2, &mut write1);

        tokio::select! {
            result = copy1 => {
                if let Err(e) = result {
                    error!("Error copying data: {}", e);
                }
            }
            result = copy2 => {
                if let Err(e) = result {
                    error!("Error copying data: {}", e);
                }
            }
        }
    });
}
