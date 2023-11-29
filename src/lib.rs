pub mod http;
pub mod socks;
pub mod utils;

mod config;
mod connection;

use std::sync::Arc;

use clap::Parser;
pub use config::Config;
use log::debug;
use tokio::net::TcpListener;

use crate::connection::Connection;

pub struct ProxyServer;

impl ProxyServer {
    async fn handle_connection(connection: Connection) {
        // socks5 代理最大的范围是 1 + 1 + 255，所以仅读前257字节即可区分协议类型
        let mut buf = [0; 257];
        let size = connection.client.peek(&mut buf).await.unwrap();
        let messsage = &buf[..size];

        debug!("receive first buffer, size: {size}, {:?}", &buf[..size]);

        if socks::is_socks5_proxy(messsage) {
            debug!("use as socks5 proxy");
            socks::handle(connection).await;
        } else {
            debug!("use as http proxy");
            http::handle(connection).await;
        }
    }
    async fn run(config: Config) {
        let config = Arc::new(config);
        let addr = format!("{}:{}", config.host, config.port);
        let server = TcpListener::bind(&addr)
            .await
            .expect("launch proxy server failed");

        println!("proxy server is running at: {addr}");
        println!("proxy address:");
        println!("\t\thttp://{addr}");
        println!("\t\t{}", socks::format_socks5_info(&config));

        loop {
            let (client, _) = server.accept().await.unwrap();
            debug!("new client connected");

            let connection = Connection {
                client,
                config: Arc::clone(&config),
            };
            tokio::spawn(async move {
                ProxyServer::handle_connection(connection).await;
            });
        }
    }
}

pub async fn launch_from_cli() {
    let config = Config::parse();
    ProxyServer::run(config).await;
}
