use std::{
    io::{prelude::*, Write},
    net::{TcpListener, TcpStream},
    sync::Arc,
    thread,
};

use log::debug;
use rust_proxy::{socks, utils, ThreadPool};

fn is_connect(info: &String) -> bool {
    let lines: Vec<_> = info.split("\r\n").collect();
    let first: Vec<_> = lines[0].split(" ").collect();
    first[0] == "CONNECT"
}

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

fn handle_connection(mut client: TcpStream, addr: Arc<&str>) {
    let mut buf = [0; 8192];
    let size = client.read(&mut buf).unwrap();
    let str = String::from_utf8_lossy(&buf).to_string();
    let mut server;

    let target = parse_target_server(&str);

    debug!("target server: {target}, size: {size}, {:?}", &buf[..size]);

    if target.is_empty() {
        // 解析目标服务器出错，尝试作为socks代理处理
        socks::handle(&buf[..size], client, addr);
        return;
    }

    if is_connect(&str) {
        server = TcpStream::connect(target).unwrap();

        // 连接目标成功之后，返回下面内容，表示 通知浏览器连接成功
        client
            .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            .unwrap();
    } else {
        server = TcpStream::connect(target).unwrap();
        server.write(&buf[..size]).unwrap();
    }

    //  经过实践，此处无法使用Arc引用计数来跨线程, TcpStream::read会卡住
    let mut send_server = server.try_clone().unwrap();
    let mut recv_client = client.try_clone().unwrap();
    thread::spawn(move || {
        // 把服务器的请求转发给客户端
        utils::pipe(&mut send_server, &mut recv_client);
    });
    // 把客户端的请求转发给服务器
    utils::pipe(&mut client, &mut server);
}

fn main() {
    // 注意，env_logger 必须尽可能早的初始化
    env_logger::init();
    let addr = Arc::new("127.0.0.1:7878");
    let server = TcpListener::bind(addr.as_ref()).expect("launch server failed");
    let pool: ThreadPool = ThreadPool::with_capacity(4);
    for connection in server.incoming() {
        if let Ok(connection) = connection {
            debug!(
                "new connection received: {}",
                connection.peer_addr().unwrap()
            );

            let addr = Arc::clone(&addr);
            pool.run(|| {
                handle_connection(connection, addr);
            });
        }
    }

    debug!("shutdown server");
}
