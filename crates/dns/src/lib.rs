use log::debug;
use tokio::net::UdpSocket;

mod dns;

pub struct DnsServer;

impl DnsServer {
    async fn run() {
        let server = UdpSocket::bind("127.0.0.1:7878").await.unwrap();
        debug!("run dns server");

        loop {
            let mut buf = [0; 1024];
            let (size, from) = server.recv_from(&mut buf).await.unwrap();

            debug!("receive buf: {size}, {:?}", &buf[..size]);

            let data = dns::resolve(&buf[..size]).await;

            debug!("reply: {data:?}");
            server.send_to(&data, from).await.unwrap();
        }
    }
}

pub async fn launch() {
    DnsServer::run().await;
}
