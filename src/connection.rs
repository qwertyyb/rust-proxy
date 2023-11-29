use std::sync::Arc;

use tokio::net::TcpStream;

use crate::Config;

/// 每个客户端连接代理服务器时，此结构体对象会传给 http 或 socks5 代理的 handle 方法
pub struct Connection {
    /// 每个客户端的连接
    pub client: TcpStream,

    /// 启动代理服务器的配置
    pub config: Arc<Config>,
}
