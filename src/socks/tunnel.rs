use log::debug;
use rand::{thread_rng, Rng};
use std::{
    collections::HashMap,
    io::{prelude::*, ErrorKind},
    net::{TcpStream, UdpSocket},
    sync::{Mutex, OnceLock},
    thread,
};

use crate::socks::{constant::ATYP, utils};

pub static NAT: OnceLock<Mutex<Nat>> = OnceLock::new();

pub struct Nat {
    map: HashMap<u16, u16>,
}
impl Nat {
    pub fn global() -> &'static Mutex<Self> {
        NAT.get_or_init(|| {
            Mutex::new(Nat {
                map: HashMap::new(),
            })
        })
    }
    pub fn select_port(&self) -> Result<u16, &str> {
        let mut rng = thread_rng();
        for _i in 0..10 {
            let port = rng.gen_range(50000..=60000);
            if self.map.contains_key(&port) {
                continue;
            }
            if self.map.values().any(|value| value == &port) {
                continue;
            }
            return Ok(port);
        }
        return Err("cant find avaliable port");
    }
    fn insert(&mut self, local_port: u16, remote_port: u16) {
        self.map.insert(local_port, remote_port);
    }
}

pub struct UdpTunnel {
    pub local_socket: UdpSocket,
    pub remote_socket: UdpSocket,
}

impl UdpTunnel {
    pub fn new() -> Self {
        Self {
            local_socket: Self::create_socket(),
            remote_socket: Self::create_socket(),
        }
    }

    pub fn create_socket() -> UdpSocket {
        let port = Nat::global().lock().unwrap().select_port().unwrap();
        // 1.1 使用UdpSocket::bind方法创建本地udp_socket
        let socket = UdpSocket::bind(format!("0.0.0.0:{port}")).unwrap();
        return socket;
    }
    pub fn start(&self) {
        let nat = Nat::global();
        // 如果客户端传入了客户端将要使用的
        let mut buf = [0; 10240];
        let (size, client_addr) = self.local_socket.recv_from(&mut buf).expect("错误");
        debug!("size: {size}");
        self.local_socket
            .connect(client_addr)
            .expect("connect udp local client failed");

        let atyp = ATYP::from(buf[3]);
        let (target, next_pos) = utils::parse_target(atyp, &buf[4..size]);
        let next_pos = next_pos + 4;

        self.remote_socket
            .connect(target)
            .expect("connect udp remote server failed");

        // 监听数据开始交换

        let client = self.local_socket.try_clone().unwrap();
        let server = self.remote_socket.try_clone().unwrap();
        thread::spawn(move || {
            utils::pipe_udp_to_server(&client, &server);
        });

        let client = self.local_socket.try_clone().unwrap();
        let server = self.remote_socket.try_clone().unwrap();
        thread::spawn(move || {
            utils::pipe_udp_to_client(&server, &client);
        });

        self.remote_socket
            .send(&buf[next_pos..size])
            .expect("send udp data to server failed");

        // 3. 添加到 hashmap 映射一下
        nat.lock().unwrap().insert(
            self.local_socket.local_addr().unwrap().port(),
            self.remote_socket.local_addr().unwrap().port(),
        );
    }
}
