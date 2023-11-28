mod config;
pub mod http;
pub mod socks;
pub mod utils;

pub use config::Config;
use log::{debug, info};
use tokio::net::{TcpListener, TcpStream};

async fn handle_client(client: TcpStream) {
    let mut buf = [0; 8192];
    let size = client.peek(&mut buf).await.unwrap();
    let messsage = &buf[..size];

    debug!("receive first buffer, size: {size}, {:?}", &buf[..size]);

    if socks::is_socks5_proxy(messsage) {
        debug!("use as socks5 proxy");
        socks::handle(client).await;
    } else {
        debug!("use as http proxy");
        http::handle(client).await;
    }
}

pub async fn launch() {
    let config = Config::global();
    info!("config: {config:#?}");
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
        tokio::spawn(async move {
            handle_client(client).await;
        });
    }
}
