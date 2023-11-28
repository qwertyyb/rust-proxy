use std::sync::OnceLock;

use log::{debug, info, warn};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::config::Config;

use super::constant::Method;

static AUTH: OnceLock<Auth> = OnceLock::new();

pub struct Auth<'a> {
    config: &'a Config,
}

impl Auth<'_> {
    fn need_auth(&self) -> bool {
        self.config.has_auth()
    }
    fn check_auth(&self, username: &String, password: &String) -> bool {
        self.config.has_auth()
            && *self.config.username.as_ref().unwrap() == *username
            && *self.config.password.as_ref().unwrap() == *password
    }
    async fn handle_user_pwd(&self, client: &mut TcpStream) -> Result<bool, &str> {
        //    client -> proxy
        //    +----+------+----------+------+----------+
        //    |VER | ULEN |  UNAME   | PLEN |  PASSWD  |
        //    +----+------+----------+------+----------+
        //    | 1  |  1   | 1 to 255 |  1   | 1 to 255 |
        //    +----+------+----------+------+----------+
        let mut buf = [0; 513];
        let _ = client.read(&mut buf).await;
        if buf[0] != 0x01 {
            return Err("auth version not support");
        }
        let ulen = buf[1] as usize;
        let uname = String::from_utf8_lossy(&buf[2..(2 + ulen)]).to_string();
        let plen = buf[2 + ulen] as usize;
        let passwd =
            String::from_utf8_lossy(&buf[(2 + ulen + 1)..(2 + ulen + 1 + plen)]).to_string();
        debug!("auth, username: {uname:?}, password: {passwd:?}");

        //    proxy -> client
        //   +----+--------+
        //   |VER | STATUS |
        //   +----+--------+
        //   | 1  |   1    |
        //   +----+--------+
        if self.check_auth(&uname, &passwd) {
            info!("auth successfully");
            client.write_all(&[0x01, 0x00]).await.unwrap();
            return Ok(true);
        }
        warn!("auth failed");
        client.write_all(&[0x01, 0x01]).await.unwrap();
        client.shutdown().await.unwrap();
        return Ok(false);
    }
    pub async fn handle(&self, message: &[u8], client: &mut TcpStream) -> Result<bool, &str> {
        //      client -> proxy
        //     +----+----------+----------+
        //     |VER | NMETHODS | METHODS  |
        //     +----+----------+----------+
        //     | 1  |    1     | 1 to 255 |
        //     +----+----------+----------+

        //      proxy -> server
        //     +----+--------+
        //     |VER | METHOD |
        //     +----+--------+
        //     | 1  |   1    |
        //     +----+--------+
        let [_, nmethods, ..] = *message else {
            return Ok(false);
        };
        let mut methods = message[2..2 + (nmethods as usize)]
            .into_iter()
            .map(|value| Method::from(*value));
        if self.need_auth() {
            info!("proxy server need username/password auth");
            if methods.any(|value| value == Method::UserPwd) {
                debug!("client support username/password auth, start auth");
                client
                    .write_all(&[0x05, Method::UserPwd as u8])
                    .await
                    .unwrap();
                return self.handle_user_pwd(client).await;
            } else {
                client.write_all(&[0x05, 0xff]).await.unwrap();
                return Ok(false);
            }
        } else if !Auth::global().need_auth() && methods.any(|value| value == Method::None) {
            info!("proxy server dont need username/password auth");
            async { client.write_all(&[0x05, Method::None as u8]).await }
                .await
                .unwrap();
            return Ok(true);
        } else {
            return Err("unsupport client");
        }
    }
    pub fn global() -> &'static Self {
        AUTH.get_or_init(|| Self {
            config: Config::global(),
        })
    }
}
