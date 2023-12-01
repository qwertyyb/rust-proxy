use log::debug;
use rand::{thread_rng, Rng};
use std::{
    collections::{HashMap, HashSet},
    io::Error,
    sync::{Arc, Mutex, OnceLock},
};
use tokio::{
    net::UdpSocket,
    task::{self, JoinHandle},
};

use crate::socks::utils::{self, parse_udp_frame};

static NAT: OnceLock<Mutex<Nat>> = OnceLock::new();

#[derive(Debug)]
struct Nat {
    ports: HashSet<u16>,
    forwards: HashMap<u16, u16>,
}
impl Nat {
    pub fn global() -> &'static Mutex<Self> {
        NAT.get_or_init(|| {
            Mutex::new(Nat {
                ports: HashSet::new(),
                forwards: HashMap::new(),
            })
        })
    }
    pub fn select_port(&mut self) -> Result<u16, &str> {
        let mut rng = thread_rng();
        for _i in 0..10 {
            let port = rng.gen_range(50000..=60000);
            if self.ports.contains(&port) {
                continue;
            }
            self.ports.insert(port);
            return Ok(port);
        }
        return Err("cant find avaliable port");
    }
    fn insert(&mut self, local_port: u16, remote_port: u16) {
        self.forwards.insert(local_port, remote_port);
    }
    fn remove(&mut self, k: &u16) {
        if let Some(value) = self.forwards.get(k) {
            // 如果已经存了value, 把value也从ports中删除
            self.ports.remove(value);
        }
        self.forwards.remove(k);
        self.ports.remove(k);
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
            local_socket: Arc::new(Self::create_socket().await.unwrap()),
            remote_socket: Arc::new(Self::create_socket().await.unwrap()),
            client_to_server: None,
            server_to_client: None,
        }
    }

    async fn create_socket() -> Result<UdpSocket, Error> {
        let port = Nat::global().lock().unwrap().select_port().unwrap();
        let socket = UdpSocket::bind(format!("0.0.0.0:{port}")).await;
        match socket {
            Ok(socket) => Ok(socket),
            Err(err) => {
                // 创建socket失败，释放端口
                Nat::global().lock().unwrap().remove(&port);
                Err(err)
            }
        }
    }

    pub async fn start(&mut self) {
        // 接收来自client的第一帧数据，主要用于获取发送的地址，绑定到代理服务器的sockets上，后面就不用往客户端发数据时，就无须再指定地址了。
        let mut buf = [0; 10240];
        let (size, client_addr) = self.local_socket.recv_from(&mut buf).await.expect("错误");
        let (_, target, data) = parse_udp_frame(&buf[..size]);

        debug!(
            "receive first udp data, size: {size} from : {:?}",
            client_addr
        );

        self.local_socket
            .connect(client_addr)
            .await
            .expect("connect udp local client failed");

        // 绑定交换 udp 数据
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

        // 把第一帧的udp数据转发给server
        self.remote_socket
            .send_to(data, target)
            .await
            .expect("send udp data to server failed");

        // 添加Nat映射
        Nat::global().lock().unwrap().insert(
            self.local_socket.local_addr().unwrap().port(),
            self.remote_socket.local_addr().unwrap().port(),
        );

        debug!("nat: {:?}", Nat::global().lock().unwrap());
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
