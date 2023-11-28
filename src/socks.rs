mod auth;
mod constant;
mod tunnel;
mod utils;

use std::net::IpAddr;

use log::{debug, info};
use tokio::net::TcpStream;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::socks::auth::Auth;
use crate::Config;
use crate::{
    socks::constant::{ATYP, REP},
    utils as root_utils,
};

use self::{constant::CMD, tunnel::UdpTunnel};

pub fn is_socks5_proxy(message: &[u8]) -> bool {
    let [ver, nmethods, ..] = *message else {
        return false;
    };
    if ver == 5 && message.len() == (nmethods as usize) + 2 {
        return true;
    }
    return false;
}

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
        Ok(server) => {
            // 回复客户端连接已建立
            client.write_all(&data).await.unwrap();

            root_utils::exchange(client, server).await;
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

pub async fn handle(mut client: TcpStream) {
    //     +----+----------+----------+
    //     |VER | NMETHODS | METHODS  |
    //     +----+----------+----------+
    //     | 1  |    1     | 1 to 255 |
    //     +----+----------+----------+

    let mut buf = [0; 257];
    let size = client.read(&mut buf).await.unwrap();
    let message = &buf[..size];
    info!("receive first from client, size: {size}, message: {message:?}");
    let success = Auth::global().handle(message, &mut client).await.unwrap();
    if !success {
        return;
    }

    let mut buf = [0; 1024];
    let size = client.read(&mut buf).await.unwrap();

    let (target, _) = utils::parse_target(ATYP::from(buf[3]), &buf[4..size]);
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

    return;
}
