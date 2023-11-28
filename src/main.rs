use std::env;
use std::error::Error;

use log::debug;
use rust_proxy::config::Config;
use rust_proxy::socks::is_socks5_proxy;
use rust_proxy::{http, socks};

use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};

async fn handle_client(mut client: TcpStream) -> Result<(), Box<dyn Error>> {
    let mut buf = [0; 8192];
    let size = client.read(&mut buf).await?;
    let messsage = &buf[..size];

    debug!("receive first buffer, size: {size}, {:?}", &buf[..size]);

    if is_socks5_proxy(messsage) {
        debug!("use as socks5 proxy");
        socks::handle(messsage, client).await;
        return Ok(());
    } else {
        debug!("use as http proxy");
        http::handle(messsage, client).await;
    }
    return Ok(());
}

#[tokio::main]
async fn main() {
    // 注意，env_logger 必须尽可能早的初始化
    env_logger::init();
    let config = Config::global();
    debug!("launch arguments: {:?}, config: {:?}", env::args(), config);
    let addr = format!("{}:{}", config.host, config.port);
    let server = TcpListener::bind(&addr)
        .await
        .expect("launch proxy server failed");

    println!("proxy server is running at: {addr}");
    println!("proxy address:");
    println!("\t\thttp://{addr}");

    let mut socks5_info = String::from("socks5://");
    if let (Some(username), Some(password)) =
        (config.username.as_deref(), config.password.as_deref())
    {
        socks5_info.push_str(&format!("{username}:{password}@"));
    }
    socks5_info.push_str(&addr);
    println!("\t\t{socks5_info}");

    loop {
        let (client, _) = server.accept().await.unwrap();
        debug!("new client connected");
        tokio::spawn(async move {
            handle_client(client).await.unwrap();
        });
    }
}
