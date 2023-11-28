mod constant;
mod tunnel;
mod utils;

use std::net::IpAddr;

use log::{debug, info};
use tokio::net::TcpStream;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::config::Config;
use crate::{
    socks::constant::{ATYP, REP},
    utils as root_utils,
};

use self::{
    constant::{Method, CMD},
    tunnel::UdpTunnel,
};

pub fn is_socks5_proxy(message: &[u8]) -> bool {
    let [ver, nmethods, ..] = *message else {
        return false;
    };
    if ver == 5 && message.len() == (nmethods as usize) + 2 {
        return true;
    }
    return false;
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

async fn handle_auth(client: &mut TcpStream) -> bool {
    let mut buf = [0; 513];
    let _ = client.read(&mut buf).await;
    if buf[0] != 0x01 {
        panic!("auth failed, ver: {}", buf[0]);
    }
    let ulen = buf[1] as usize;
    let uname = String::from_utf8_lossy(&buf[2..(2 + ulen)]).to_string();
    let plen = buf[2 + ulen] as usize;
    let pass = String::from_utf8_lossy(&buf[(2 + ulen + 1)..(2 + ulen + 1 + plen)]).to_string();
    debug!("auth, username: {uname:?}, password: {pass:?}");

    let config = Config::global();
    if uname == *config.username.as_ref().unwrap() && pass == *config.password.as_ref().unwrap() {
        debug!("auth successfully");
        client.write_all(&[0x01, 0x00]).await.unwrap();
        return true;
    }
    debug!("auth failed");
    client.write_all(&[0x01, 0x01]).await.unwrap();
    client.shutdown().await.unwrap();
    return false;
}

pub async fn handle(message: &[u8], mut client: TcpStream) {
    //     +----+----------+----------+
    //     |VER | NMETHODS | METHODS  |
    //     +----+----------+----------+
    //     | 1  |    1     | 1 to 255 |
    //     +----+----------+----------+

    let [_, nmethods, ..] = *message else {
        return;
    };
    let mut methods = message[2..2 + (nmethods as usize)]
        .into_iter()
        .map(|value| Method::from(*value));

    if Config::global().need_auth() {
        info!("proxy server need username/password auth");
        if methods.any(|value| value == Method::UserPwd) {
            debug!("client support username/password auth, start auth");
            client
                .write_all(&[0x05, Method::UserPwd as u8])
                .await
                .unwrap();
            if !handle_auth(&mut client).await {
                return;
            }
        } else {
            client.write_all(&[0x05, 0xff]).await.unwrap();
            return;
        }
    } else if !Config::global().need_auth() && methods.any(|value| value == Method::None) {
        info!("proxy server dont need username/password auth");
        async { client.write_all(&[0x05, Method::None as u8]).await }
            .await
            .unwrap();
    } else {
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
