mod auth;
mod constant;
mod tunnel;
mod utils;

use std::net::IpAddr;

use log::{debug, info};
use tokio::net::TcpStream;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::connection::Connection;
use crate::socks::constant::{ATYP, REP};
use crate::Config;

use self::{constant::CMD, tunnel::UdpTunnel};

fn parse_cmd(message: &[u8]) -> CMD {
    CMD::from(message[1])
}

async fn handle_connect(mut client: TcpStream, target: &String) {
    let cur_addr = client.local_addr().unwrap();
    let mut data = vec![0x05, REP::Succeeded as u8, 0x00, ATYP::from(cur_addr) as u8];
    match cur_addr.ip() {
        IpAddr::V4(ip) => {
            data.extend_from_slice(&ip.octets());
        }
        IpAddr::V6(ip) => {
            data.extend_from_slice(&ip.octets());
        }
    }
    data.extend_from_slice(&cur_addr.port().to_be_bytes());

    let server = TcpStream::connect(target).await;
    match server {
        Ok(mut server) => {
            // 回复客户端连接已建立
            client.write_all(&data).await.unwrap();

            tokio::io::copy_bidirectional(&mut client, &mut server)
                .await
                .unwrap();
        }
        Err(error) => {
            data[1] = REP::from(error.kind()) as u8;
            let _ = client.write_all(&data);
        }
    }
}

async fn handle_udp(mut client: TcpStream) {
    // target为客户端的IP和端口号
    // 返回要使用的UDP的端口
    let cur_addr = client.local_addr().unwrap();
    let mut data = vec![0x05, REP::Succeeded as u8, 0x00, ATYP::from(cur_addr) as u8];
    match cur_addr.ip() {
        IpAddr::V4(ip) => {
            data.extend_from_slice(&ip.octets());
        }
        IpAddr::V6(ip) => {
            data.extend_from_slice(&ip.octets());
        }
    }
    let mut udp_tunnel = UdpTunnel::new().await;
    let local_port = udp_tunnel.local_socket.local_addr().unwrap().port();
    data.extend_from_slice(&local_port.to_be_bytes());
    client.write_all(&data).await.unwrap();

    udp_tunnel.start().await;

    let mut buf = [0; 1024];
    match client.read(&mut buf).await {
        Ok(0) => drop(udp_tunnel),
        Ok(_) => {}
        Err(_) => drop(udp_tunnel),
    }
}

/// 根据 `Config` 输出代理 socks5 代理地址
pub fn format_socks5_info(config: &Config) -> String {
    let mut socks5_info = String::from("socks5://");
    if let (Some(username), Some(password)) =
        (config.username.as_deref(), config.password.as_deref())
    {
        socks5_info.push_str(&format!("{username}:{password}@"));
    }
    socks5_info.push_str(&format!("{}:{}", config.host, config.port));
    socks5_info
}

/// 根据前几个字节，判断是否是 socks5 代理协议
pub fn is_socks5_proxy(message: &[u8]) -> bool {
    //     +----+----------+----------+
    //     |VER | NMETHODS | METHODS  |
    //     +----+----------+----------+
    //     | 1  |    1     | 1 to 255 |
    //     +----+----------+----------+
    let [ver, nmethods, ..] = *message else {
        return false;
    };
    if ver == 5 && message.len() == (nmethods as usize) + 2 {
        true
    } else {
        false
    }
}

/// socks5 流量代理方法，每个代理进入时会走到此方法
pub async fn handle(connection: Connection) {
    //     +----+----------+----------+
    //     |VER | NMETHODS | METHODS  |
    //     +----+----------+----------+
    //     | 1  |    1     | 1 to 255 |
    //     +----+----------+----------+
    let Connection { mut client, config } = connection;

    let mut buf = [0; 257];
    let size = client.read(&mut buf).await.unwrap();
    let message = &buf[..size];
    info!("receive first from client, size: {size}, message: {message:?}");

    let success = auth::handle(&config, message, &mut client).await.unwrap();
    if !success {
        return;
    }

    let mut buf = [0; 1024];
    let size = client.read(&mut buf).await.unwrap();

    let (target, _) = utils::parse_target(&ATYP::from(buf[3]), &buf[4..size]);
    let cmd = parse_cmd(&buf[..size]);

    debug!(
        "received from client: {size}, cmd: {cmd:?}, target: {target}, raw: {:?}",
        &buf[..size]
    );

    match cmd {
        CMD::CONNECT => {
            handle_connect(client, &target).await;
        }
        CMD::UDP => {
            handle_udp(client).await;
        }
        CMD::BIND | CMD::UNKOWN => {
            debug!("unkown cmd");
        }
    }
}
