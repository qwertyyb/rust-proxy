mod constant;
mod tunnel;
mod utils;

use std::{
    io::{Read, Write},
    net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs},
    str::FromStr,
    sync::Arc,
    thread,
};

use log::debug;

use crate::{
    socks::constant::{ATYP, REP},
    utils as root_utils,
};

use self::{
    constant::{Method, CMD},
    tunnel::UdpTunnel,
};

fn is_socks5_proxy(message: &[u8]) -> bool {
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

fn handle_connect(mut client: TcpStream, target: &String, addr: Arc<&str>) {
    let server = TcpStream::connect(target);

    let cur_addr = addr
        .as_ref()
        .to_socket_addrs()
        .unwrap()
        .collect::<Vec<SocketAddr>>();
    let cur_addr = cur_addr.first().unwrap();
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

    match server {
        Ok(mut server) => {
            // 回复客户端连接已建立
            let _ = client.write_all(&data);

            // 双方可互发数据了
            let mut send_client = client.try_clone().unwrap();
            let mut recv_server = server.try_clone().unwrap();
            thread::spawn(move || root_utils::pipe(&mut send_client, &mut recv_server));

            root_utils::pipe(&mut server, &mut client);
        }
        Err(error) => {
            data[1] = REP::from(error.kind()) as u8;
            let _ = client.write_all(&data);
        }
    }
}

fn handle_udp(mut client: TcpStream, addr: Arc<&str>) {
    // target为客户端的IP和端口号
    // 返回要使用的UDP的端口
    let cur_addr = SocketAddr::from_str(&addr).unwrap();
    let mut data = vec![
        0x05,
        REP::Succeeded as u8,
        0x00,
        ATYP::from(&cur_addr) as u8,
    ];
    match cur_addr.ip() {
        IpAddr::V4(ip) => {
            data.extend_from_slice(&ip.octets());
        }
        IpAddr::V6(ip) => {
            data.extend_from_slice(&ip.octets());
        }
    }
    let udp_tunnel = UdpTunnel::new();
    let local_port = udp_tunnel.local_socket.local_addr().unwrap().port();
    data.extend_from_slice(&local_port.to_be_bytes());
    client.write_all(&data).unwrap();

    udp_tunnel.start();
}

pub fn handle(message: &[u8], mut client: TcpStream, addr: Arc<&str>) -> bool {
    if !is_socks5_proxy(message) {
        return false;
    }
    debug!("socks5 proxy client connected");

    let [_, nmethods, ..] = *message else {
        return false;
    };
    let mut methods = message[2..2 + (nmethods as usize)]
        .into_iter()
        .map(|value| Method::from(*value));

    if methods.any(|value| value == Method::UserPwd) {
        debug!("start username/password auth");
        client.write_all(&[0x05, Method::UserPwd as u8]).unwrap();

        let mut buf = [0; 513];
        let _ = client.read(&mut buf).unwrap();
        if buf[0] != 0x01 {
            panic!("auto failed, ver: {}", buf[0]);
        }
        let ulen = buf[1] as usize;
        let uname = String::from_utf8_lossy(&buf[2..(2 + ulen)]).to_string();
        let plen = buf[2 + ulen] as usize;
        let pass = String::from_utf8_lossy(&buf[(2 + ulen + 1)..(2 + ulen + 1 + plen)]).to_string();
        debug!("auth, username: {uname}, password: {pass}");

        if uname == "hello" && pass == "world" {
            debug!("auth successfully");
            client.write_all(&[0x01, 0x00]).unwrap();
        } else {
            let _ = client.write_all(&[0x01, 0x01]);
            let _ = client.shutdown(std::net::Shutdown::Both);
            return true;
        }
    } else if methods.any(|value| value == Method::None) {
        client.write_all(&[0x05, Method::None as u8]).unwrap();
    }

    let mut buf = [0; 1024];
    let size = client.read(&mut buf).unwrap();

    let (target, _) = utils::parse_target(ATYP::from(buf[3]), &buf[4..size]);
    let cmd = parse_cmd(&buf[..size]);

    debug!(
        "received from client: {size}, cmd: {cmd:?}, target: {target}, raw: {:?}",
        &buf[..size]
    );

    match cmd {
        CMD::CONNECT => {
            handle_connect(client, &target, addr);
        }
        CMD::UDP => {
            handle_udp(client, addr);
        }
        CMD::BIND | CMD::UNKOWN => {
            debug!("unkown cmd");
        }
    }
    // }
    return true;
}
