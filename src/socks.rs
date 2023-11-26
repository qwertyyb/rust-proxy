mod constant;
mod utils;

use std::{net::{TcpStream, UdpSocket, ToSocketAddrs, SocketAddr, IpAddr}, io::{Write, Read}, thread, sync::{Arc}};

use crate::{utils as root_utils, socks::{constant::{ATYP, REP}, utils::{pipe_udp_to_server, pipe_udp_to_client}}};

use self::constant::{Method, CMD};

fn is_socks5_proxy(message: &[u8]) -> bool {
  let [ver, nmethods, ..] = *message else {
    return false;
  };
  if ver == 5 && message.len() == (nmethods as usize) + 2 {
      return true;
  }
  return false
}

fn parse_cmd(message: &[u8]) -> CMD {
  CMD::from(message[1])
}

fn handle_connect(mut client: TcpStream, target: &String, cmd: &CMD, addr: Arc<&str>) {
  let server = TcpStream::connect(target);

  let cur_addr = addr.as_ref()
    .to_socket_addrs().unwrap()
    .collect::<Vec<SocketAddr>>();
  let cur_addr = cur_addr
    .first()
    .unwrap();
  let mut data = vec![
    0x05, REP::Succeeded as u8, 0x00, ATYP::from(cur_addr) as u8,
  ];
  match cur_addr.ip() {
    IpAddr::V4(ip) => {
      data.extend_from_slice(&ip.octets());
    },
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
      thread::spawn(move || {
        root_utils::pipe(&mut send_client, &mut recv_server)
      });

      root_utils::pipe(&mut server, &mut client);
    },
    Err(error) => {
      data[1] = REP::from(error.kind()) as u8;
      let _ = client.write_all(&data);
    }
  }
}

fn handle_UDP(mut client: TcpStream, mut udp_socket: UdpSocket, target: &String, addr: Arc<&str>) {
  // target为客户端的IP和端口号
  // 返回要使用的UDP的端口
  let cur_addr = udp_socket.local_addr().unwrap();
  let mut data = vec![
    0x05, REP::Succeeded as u8, 0x00, ATYP::from(&cur_addr) as u8,
  ];
  match cur_addr.ip() {
    IpAddr::V4(ip) => {
      data.extend_from_slice(&ip.octets());
    },
    IpAddr::V6(ip) => {
      data.extend_from_slice(&ip.octets());
    }
  }
  data.extend_from_slice(&cur_addr.port().to_be_bytes());

  client.write_all(&data).unwrap();

  println!("start app");
  
  let mut buf = [0; 10240];
  let (size, client_addr) = udp_socket.recv_from(&mut buf).expect("错误");

  let atyp = ATYP::from(buf[3]);
  let (target, next_pos) = utils::parse_target(atyp, &buf[4..size]);
  let next_pos = next_pos + 4;

  println!("received: target: {target}, client_addr: {client_addr}, size: {size}, raw: {:?}, data_pos: {}, data: {:?},", &buf[..size], next_pos + 4, &buf[next_pos..size]);

  let mut server_socket = udp_socket.try_clone().unwrap();
  // server_socket.connect(&target).expect("connect remote failed: {target}");
  server_socket.send_to(&buf[next_pos..size], target).expect("send udp failed");
  // udp_socket.connect(client_addr).expect("connect client failed");

  let mut client = udp_socket.try_clone().unwrap();
  let mut server = server_socket.try_clone().unwrap();
  thread::spawn(move || {
    pipe_udp_to_server(&mut client, &mut server);
  });
  thread::spawn(move || {
    pipe_udp_to_client(&mut server_socket, &mut udp_socket);
  });
}

pub fn handle(message: &[u8], mut client: TcpStream, udp_socket: UdpSocket, addr: Arc<&str>) -> bool {
  if !is_socks5_proxy(message) {
    return false;
  }
  println!("socks5 proxy client connected");

  let [_, nmethods, ..] = *message else {
      return false;
  };
  let mut methods = message[2..2 + (nmethods as usize)]
    .into_iter()
    .map(|value| Method::from(*value));

  // 先实现没有认证的socks5代理
  if methods.any(|value| value == Method::None) {
    client.write_all(&[0x05, Method::None as u8]).unwrap();
    
    let mut buf = [0; 1024];
    let size = client.read(&mut buf).unwrap();

    let (target, _) = utils::parse_target(ATYP::from(buf[3]), &buf[4..size]);
    let cmd = parse_cmd(&buf[..size]);

    println!("received from client: {size}, cmd: {cmd:?}, target: {target}, raw: {:?}", &buf[..size]);

    match cmd {
      CMD::CONNECT => {
        handle_connect(client, &target, &cmd, addr);
      },
      CMD::UDP => {
        handle_UDP(client, udp_socket, &target, addr);
      },
      CMD::BIND | CMD::UNKOWN => {
        println!("unkown cmd");
      }
    }
  }
  return true;
}