use std::{
    fs,
    net::{IpAddr, SocketAddr},
    sync::{Arc, OnceLock},
};

use log::{debug, error, warn};
use tokio::{
    net::UdpSocket,
    sync::{broadcast, Mutex},
    task::JoinHandle,
};

use crate::dns::{hosts::Hosts, Frame};

struct UpstreamServer {
    servers: Vec<SocketAddr>,
    socket: Arc<UdpSocket>,
    listen_job: Option<JoinHandle<()>>,
    data_sender: broadcast::Sender<Vec<u8>>,
    data_receiver: broadcast::Receiver<Vec<u8>>,
}

impl UpstreamServer {
    fn load_upstream_server(&mut self) {
        let result = fs::read_to_string("/etc/resolv.conf");
        match result {
            Ok(content) => content.lines().for_each(|line| {
                let line = line.trim();
                if line.is_empty() || line.starts_with("#") {
                    return;
                }
                let parts: Vec<&str> = line.split_ascii_whitespace().collect();
                if parts.len() >= 2 && parts[0] == "nameserver" {
                    let addr: Result<IpAddr, _> = parts[1].parse();
                    if let Ok(addr) = addr {
                        self.servers.push(SocketAddr::new(addr, 53));
                    } else {
                        warn!("parse nameserver failed: {line}");
                    }
                }
            }),
            Err(err) => {
                error!("load /etc/resolv.conf failed: {err}");
            }
        }
    }
    fn listen_data(&self) -> JoinHandle<()> {
        let socket = Arc::clone(&self.socket);
        let sender = self.data_sender.clone();
        tokio::spawn(async move {
            loop {
                let mut buf = [0; 10240];
                match socket.recv_from(&mut buf).await {
                    Ok((size, _)) => {
                        sender.send(buf[..size].to_vec()).unwrap();
                    }
                    Err(err) => error!("resolve from upstream failed: {err}"),
                };
            }
        })
    }
    fn new() -> Self {
        let (sender, receiver) = broadcast::channel(10);
        let socket = std::net::UdpSocket::bind("0.0.0.0:6767").unwrap();
        socket.set_nonblocking(true).unwrap();
        let socket = Arc::new(UdpSocket::from_std(socket).unwrap());
        let mut me = Self {
            servers: vec![],
            socket,
            data_sender: sender,
            data_receiver: receiver,
            listen_job: None,
        };
        me.load_upstream_server();
        me.listen_job = Some(me.listen_data());
        me
    }
    pub async fn resolve<'a>(&mut self, frame: &Frame<'a>) -> Result<Vec<u8>, String> {
        debug!("resolve from upstream");
        if self.servers.is_empty() {
            warn!("upstream server is not found");
            return Err("upstream server is not found".to_string());
        }
        let first = self.servers.first().unwrap();
        self.socket
            .send_to(frame.origin.unwrap(), first)
            .await
            .unwrap();

        loop {
            match self.data_receiver.recv().await {
                Ok(data) => {
                    debug!("receive data: {data:?}");
                    if &frame.origin.unwrap()[..2] == &data[..2] {
                        return Ok(data);
                    }
                }
                Err(err) => return Err(format!("failed: {err}")),
            }
        }
    }
}

impl Drop for UpstreamServer {
    fn drop(&mut self) {
        if let Some(job) = self.listen_job.take() {
            job.abort();
        }
    }
}

static RESOLVER: OnceLock<Arc<Mutex<Resolver>>> = OnceLock::new();

struct Resolver {
    upstream: UpstreamServer,
}

impl Resolver {
    fn new() -> Self {
        Self {
            upstream: UpstreamServer::new(),
        }
    }
    pub fn global() -> &'static Arc<Mutex<Self>> {
        RESOLVER.get_or_init(|| Arc::new(Mutex::new(Self::new())))
    }
    pub async fn resolve(&mut self, data: &[u8]) -> Vec<u8> {
        let hosts = Hosts::build();

        let frame = Frame::parse(data);
        debug!("frame: {frame:?}");

        let mut answers = Vec::new();
        for question in &frame.question {
            if let Some(results) = hosts.search(question) {
                debug!("find record, {results:?}");
                answers.extend(results);
            }
        }

        if answers.is_empty() {
            // 本地未解析到结果，把报文转发给上游服务器
            let reply_frame = self.upstream.resolve(&frame).await.unwrap();
            return reply_frame;
        }

        // 生成应答报文
        let reply_frame = frame.create_reply(answers);
        return reply_frame.stringify();
    }
}

pub async fn resolve(data: &[u8]) -> Vec<u8> {
    let mut resolver = Resolver::global().lock().await;
    resolver.resolve(data).await
}
