use log::debug;
use rand::{thread_rng, Rng};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};
use tokio::{
    net::UdpSocket,
    task::{self, JoinHandle},
};

use crate::socks::{constant::ATYP, utils};

static NAT: OnceLock<Mutex<Nat>> = OnceLock::new();

struct Nat {
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
    fn remove(&mut self, k: &u16) {
        self.map.remove(k);
    }
}

pub struct UdpTunnel {
    pub local_socket: Arc<UdpSocket>,
    pub remote_socket: Arc<UdpSocket>,
    client_to_server: Option<JoinHandle<()>>,
    server_to_client: Option<JoinHandle<()>>,
}

impl UdpTunnel {
    pub async fn new() -> Self {
        Self {
            local_socket: Arc::new(Self::create_socket().await),
            remote_socket: Arc::new(Self::create_socket().await),
            client_to_server: None,
            server_to_client: None,
        }
    }

    async fn create_socket() -> UdpSocket {
        let port = Nat::global().lock().unwrap().select_port().unwrap();
        let socket = UdpSocket::bind(format!("0.0.0.0:{port}")).await.unwrap();
        return socket;
    }

    pub async fn start(&mut self) {
        let nat = Nat::global();

        let mut buf = [0; 10240];
        let (size, client_addr) = self.local_socket.recv_from(&mut buf).await.expect("错误");
        debug!("receive, size: {size}, addr: {:?}", client_addr);

        self.local_socket
            .connect(client_addr)
            .await
            .expect("connect udp local client failed");

        let atyp = ATYP::from(buf[3]);
        let (target, next_pos) = utils::parse_target(atyp, &buf[4..size]);
        let next_pos = next_pos + 4;

        self.remote_socket
            .connect(target)
            .await
            .expect("connect udp remote server failed");

        // 开始交换 udp 数据
        // 把远程 server 的数据转交给本地 client
        let remote_client = Arc::clone(&self.local_socket);
        let remote_server = Arc::clone(&self.remote_socket);
        self.server_to_client = Some(task::spawn(async {
            utils::pipe_udp_to_client(remote_server, remote_client).await;
        }));

        // 把本地 client 的数据转交给远程 server
        let local_client = Arc::clone(&self.local_socket);
        let local_server = Arc::clone(&self.remote_socket);
        self.client_to_server = Some(task::spawn(async {
            utils::pipe_udp_to_server(local_client, local_server).await;
        }));

        self.remote_socket
            .send(&buf[next_pos..size])
            .await
            .expect("send udp data to server failed");

        // 添加到 NAT 映射
        nat.lock().unwrap().insert(
            self.local_socket.local_addr().unwrap().port(),
            self.remote_socket.local_addr().unwrap().port(),
        );
    }
}

impl Drop for UdpTunnel {
    fn drop(&mut self) {
        if let Some(join) = self.client_to_server.take() {
            join.abort();
        }
        if let Some(join) = self.server_to_client.take() {
            join.abort();
        }

        // 移除 NAT 映射
        let nat = Nat::global().lock();
        if let Ok(mut nat) = nat {
            nat.remove(&self.local_socket.local_addr().unwrap().port());
        }

        debug!(
            "local port count: {}, remote port count: {}",
            Arc::strong_count(&self.local_socket),
            Arc::strong_count(&self.local_socket)
        );
    }
}
