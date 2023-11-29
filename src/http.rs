use log::debug;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::{connection::Connection, utils};

fn parse_target_server(info: &String) -> String {
    for line in info.split("\r\n") {
        let arr: Vec<_> = line.split(": ").collect();
        if arr[0] == "Host" {
            let port = if arr[1].contains(":") { "" } else { ":80" };
            return arr[1].to_string() + port;
        }
    }
    return String::new();
}

fn is_tcp(info: &String) -> bool {
    let lines: Vec<_> = info.split("\r\n").collect();
    let first: Vec<_> = lines[0].split(" ").collect();
    first[0] == "CONNECT"
}

pub async fn handle(connection: Connection) {
    let mut client = connection.client;
    let mut buf = [0; 10240];
    let size = client.read(&mut buf).await.unwrap();
    let message = &buf[..size];
    let str = String::from_utf8_lossy(message).to_string();
    let target = parse_target_server(&str);
    let mut server;
    if is_tcp(&str) {
        debug!("client is tcp");
        server = TcpStream::connect(target).await.unwrap();

        // 连接目标成功之后，返回下面内容，表示 通知浏览器连接成功
        client
            .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            .await
            .unwrap();
    } else {
        debug!("client is http");
        // http 代理
        server = TcpStream::connect(target).await.unwrap();
        server.write(&message).await.unwrap();
    }

    utils::exchange(client, server).await;
}
