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
//! 2. 库的方式
//! ```rust
//!
//! use crate::rust_proxy::ProxyServer;
//!
//! ...
//! ProxyServer::run(Config {
//!     port: 7878,
//!     host: "0.0.0.0".to_string(),
//!     username: Some("hello".to_string()),
//!     password: Some("world".to_string()),
//! })
//! .await;
//! ...
//! ```

/// http 代理模块
pub mod http;

/// socks5 代理模块
pub mod socks;

/// 每个客户端连接结构体
pub mod connection;

/// 工具方法模块
pub mod utils;

mod config;

use std::sync::Arc;

use clap::Parser;

/// 代理服务器配置
pub use config::Config;

use log::debug;
use tokio::net::TcpListener;

use crate::connection::Connection;

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

/// 自动解析命令行参数并启动代理服务器
pub async fn launch_from_cli() {
    let config = Config::parse();
    ProxyServer::run(config).await;
}
