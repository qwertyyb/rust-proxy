use log::{debug, error};
use tokio::net::TcpStream;
use tokio::task;

pub async fn exchange(mut client: TcpStream, mut server: TcpStream) {
    debug!("start exchange");
    task::spawn(async move {
        let (mut read1, mut write1) = client.split();
        let (mut read2, mut write2) = server.split();

        let client_to_server = tokio::io::copy(&mut read1, &mut write2);
        let server_to_client = tokio::io::copy(&mut read2, &mut write1);

        tokio::select! {
            result = client_to_server => {
                if let Err(e) = result {
                    error!("Error copying data: {}", e);
                }
            }
            result = server_to_client => {
                if let Err(e) = result {
                    error!("Error copying data: {}", e);
                }
            }
        }
    });
}
