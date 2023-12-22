use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use log::debug;
use tokio::net::UdpSocket;

use super::constant::ATYP;

pub fn parse_target(atyp: &ATYP, message: &[u8]) -> (String, usize) {
    let addr;
    let port_pos;
    match atyp {
        ATYP::DOMAINNAME => {
            // ATYP为域名时，第一个字节表示后面域名的长度
            let len = message[0] as usize;
            addr = String::from_utf8_lossy(&message[1..(1 + len)]).to_string();
            port_pos = 1 + len;
        }
        ATYP::IPv4 => {
            // 为IPv4时，取4个字节作为IPv4的地址
            addr = message[..4]
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<String>>()
                .join(".");
            port_pos = 4;
        }
        ATYP::IPv6 => {
            // IPv6时，后面16个字节为IPv6的地址，一共8段，每段两个字节
            addr = message[..16]
                .chunks(2)
                .map(|value| format!("{:02X}{:02X}", value[0], value[1]))
                .collect::<Vec<String>>()
                .join(":");
            port_pos = 16;
        }
        _ => {
            return (String::new(), 0);
        }
    };
    let port = u16::from_be_bytes([message[port_pos], message[port_pos + 1]]);
    (format!("{addr}:{port}"), port_pos + 2)
}

pub fn stringify_target(addr: SocketAddr) -> Vec<u8> {
    let mut data = vec![];
    match addr.ip() {
        IpAddr::V4(addr) => {
            data.push(ATYP::IPv4 as u8);
            data.extend(&addr.octets());
        }
        IpAddr::V6(addr) => {
            data.push(ATYP::IPv4 as u8);
            data.extend(&addr.octets());
        }
    }
    data.extend(addr.port().to_be_bytes());
    data
}

pub fn parse_udp_frame(frame: &[u8]) -> (ATYP, String, &[u8]) {
    let atyp = ATYP::from(frame[3]);
    let (target, next_pos) = parse_target(&atyp, &frame[4..]);
    (atyp, target, &frame[4 + next_pos..])
}

pub fn stringify_udp_frame(data: &[u8], remote_addr: SocketAddr) -> Vec<u8> {
    let mut frame: Vec<u8> = vec![0x00, 0x00, 0x00];
    frame.extend(stringify_target(remote_addr));
    frame.extend_from_slice(data);
    frame
}

pub async fn pipe_udp_to_server(current: Arc<UdpSocket>, next: Arc<UdpSocket>) {
    loop {
        let mut buf = [0; 10240];
        match current.recv(&mut buf).await {
            Ok(size) => {
                let (_, target, data) = parse_udp_frame(&buf[..size]);

                debug!(
                    "[to server] target: {target}, size: {size}, data: {:?},",
                    data
                );

                next.send_to(data, target).await.expect("send udp failed");
            }
            Err(err) => {
                debug!("udp error: {err}");
                break;
            }
        }
    }
}

pub async fn pipe_udp_to_client(current: Arc<UdpSocket>, next: Arc<UdpSocket>) {
    loop {
        let mut buf = [0; 10240];
        match current.recv(&mut buf).await {
            Ok(size) => {
                let data = stringify_udp_frame(&buf[..size], next.local_addr().unwrap());
                debug!(
                    "[to client] target: {}, size: {}",
                    next.local_addr().unwrap(),
                    data.len()
                );
                next.send(&data).await.expect("send failed");
            }
            Err(err) => {
                debug!("udp error: {err}");
                break;
            }
        }
    }
}
