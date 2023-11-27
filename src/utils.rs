use std::io::{prelude::*, ErrorKind};
use std::net::TcpStream;
use std::time::Duration;

use log::debug;

pub fn pipe(current: &mut TcpStream, next: &mut TcpStream) {
    loop {
        let _ = current.set_read_timeout(Some(Duration::from_secs(1)));
        let mut buf = [0; 10240];
        match current.read(&mut buf) {
            Ok(0) => {
                let _ = next.shutdown(std::net::Shutdown::Both);
                debug!("pipe ok");
                break;
            }
            Ok(size) => {
                next.write(&buf[..size]).expect("write failed");
            }
            Err(err) => {
                if err.kind() == ErrorKind::WouldBlock {
                    continue;
                } else {
                    debug!("Error: {err}, kind: {}", err.kind());
                    break;
                }
            }
        }
    }
}
