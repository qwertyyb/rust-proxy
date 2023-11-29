use std::sync::Arc;

use tokio::net::TcpStream;

use crate::Config;

pub struct Connection {
    pub client: TcpStream,
    pub config: Arc<Config>,
}
