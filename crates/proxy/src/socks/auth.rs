use std::sync::Arc;

use log::{debug, info, warn};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::config::Config;

use super::constant::Method;

fn check_auth(config: &Arc<Config>, username: &String, password: &String) -> bool {
    config.has_auth()
        && *config.username.as_ref().unwrap() == *username
        && *config.password.as_ref().unwrap() == *password
}
async fn handle_user_pwd(config: &Arc<Config>, client: &mut TcpStream) -> Result<bool, String> {
    //    client -> proxy
    //    +----+------+----------+------+----------+
    //    |VER | ULEN |  UNAME   | PLEN |  PASSWD  |
    //    +----+------+----------+------+----------+
    //    | 1  |  1   | 1 to 255 |  1   | 1 to 255 |
    //    +----+------+----------+------+----------+
    let mut buf = [0; 513];
    let _ = client.read(&mut buf).await;
    if buf[0] != 0x01 {
        return Err("auth version not support".to_string());
    }
    let ulen = buf[1] as usize;
    let uname = String::from_utf8_lossy(&buf[2..(2 + ulen)]).to_string();
    let plen = buf[2 + ulen] as usize;
    let passwd = String::from_utf8_lossy(&buf[(2 + ulen + 1)..(2 + ulen + 1 + plen)]).to_string();
    debug!("auth, username: {uname:?}, password: {passwd:?}");

    //    proxy -> client
    //   +----+--------+
    //   |VER | STATUS |
    //   +----+--------+
    //   | 1  |   1    |
    //   +----+--------+
    if check_auth(config, &uname, &passwd) {
        info!("auth successfully");
        client.write_all(&[0x01, 0x00]).await.unwrap();
        return Ok(true);
    }
    warn!("auth failed");
    client.write_all(&[0x01, 0x01]).await.unwrap();
    client.shutdown().await.unwrap();
    Ok(false)
}

pub async fn handle(
    config: &Arc<Config>,
    message: &[u8],
    client: &mut TcpStream,
) -> Result<bool, String> {
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
    if config.has_auth() {
        info!("proxy server need username/password auth");
        if methods.any(|value| value == Method::UserPwd) {
            debug!("client support username/password auth, start auth");
            client
                .write_all(&[0x05, Method::UserPwd as u8])
                .await
                .unwrap();
            handle_user_pwd(config, client).await
        } else {
            client.write_all(&[0x05, 0xff]).await.unwrap();
            Ok(false)
        }
    } else if !config.has_auth() && methods.any(|value| value == Method::None) {
        info!("proxy server dont need username/password auth");
        async { client.write_all(&[0x05, Method::None as u8]).await }
            .await
            .unwrap();
        Ok(true)
    } else {
        Err("unsupport client".to_string())
    }
}
