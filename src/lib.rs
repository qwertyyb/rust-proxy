//! 一个支持 http 和 socks5 代理的代理服务器
//!
//! 注意: 这里的 http 代理服务器也是支持代理 HTTPS 流量的
//!
//! 使用方式:
//!
//! 1. 命令行
//! ```bash
//! cargo run -- --port=7878 --username=hello --password=world
//! ```
//! 不加 username 和 password 参数即可启用无鉴权 socks5 代理服务器
//!

/// http 代理模块
pub mod http;

/// socks5 代理模块
pub mod socks;

/// 每个客户端连接结构体
pub mod connection;

mod config;

use std::sync::Arc;

use clap::Parser;

/// 代理服务器配置
pub use config::Config;

use dns::Frame;
use log::debug;
use tokio::net::{TcpListener, UdpSocket};

use crate::connection::Connection;

mod dns;

/// 代理服务器启动入口
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
    pub async fn run(config: Config) {
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

pub struct DnsServer;

impl DnsServer {
    async fn run() {
        let server = UdpSocket::bind("127.0.0.1:7878").await.unwrap();
        debug!("run dns server");

        let mut buf = [0; 1024];
        let (size, from) = server.recv_from(&mut buf).await.unwrap();
        debug!("receive buf: {size}, {:?}", &buf[..size]);

        let data = dns::handle(&buf[..size]);

        debug!("reply: {data:?}");
        server.send_to(&data, from).await.unwrap();
    }
}

/// 自动解析命令行参数并启动代理服务器
pub async fn launch_from_cli() {
    DnsServer::run().await;

    // let config = Config::parse();
    // ProxyServer::run(config).await;
}
