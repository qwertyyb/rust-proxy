use std::{io::ErrorKind, net::SocketAddr};

#[derive(PartialEq)]
pub enum Method {
    None = 0x00,
    GSSAPI = 0x01,
    UserPwd = 0x02,
    IANA = 0x03,   // 0x03 - 0x7E
    Priv = 0x80,   // 0x80 - 0xFE
    Refuse = 0xFF, // 没有
}

impl From<u8> for Method {
    fn from(value: u8) -> Self {
        match value {
            0 => Method::None,
            1 => Method::GSSAPI,
            2 => Method::UserPwd,
            3..=0x7E => Method::IANA,
            0x80..=0xFE => Method::Priv,
            0xFF => Method::Refuse,
            _ => Method::Refuse,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum CMD {
    CONNECT = 0x01,
    BIND = 0x02,
    UDP = 03,
    UNKOWN = 0x00,
}
impl From<u8> for CMD {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::CONNECT,
            0x02 => Self::BIND,
            0x03 => Self::UDP,
            _ => Self::UNKOWN,
        }
    }
}

pub enum REP {
    Succeeded = 0x00, // succeeded
    Failure = 0x01,   // general SOCKS server failure
    // NotAllowed = 0x02, // connection not allowed by ruleset
    NetworkUnreachable = 0x03,  // Network unreachable
    HostUnreachable = 0x04,     // Host unreachable
    ConnectionRefused = 0x05,   // Connection refused
    TTLExpired = 0x06,          // TTL expired
    CommandNotSupported = 0x07, // Command not supported
    AddressTypeNotSupported = 0x08, // Address type not supported
                                // Unknown = 0x09, // to X'FF' unassigned
}

impl From<ErrorKind> for REP {
    fn from(value: ErrorKind) -> Self {
        match value {
            ErrorKind::AddrNotAvailable => Self::HostUnreachable,
            ErrorKind::NotConnected => Self::NetworkUnreachable,
            ErrorKind::ConnectionRefused => Self::ConnectionRefused,
            ErrorKind::TimedOut => Self::TTLExpired,
            ErrorKind::InvalidInput => Self::AddressTypeNotSupported,
            ErrorKind::NotFound => Self::HostUnreachable,
            _ => Self::Failure,
        }
    }
}

#[derive(PartialEq)]
pub enum ATYP {
    IPv4 = 0x01,
    DOMAINNAME = 0x03,
    IPv6 = 0x04,
    UNKOWN = 0x00,
}
impl From<u8> for ATYP {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::IPv4,
            0x03 => Self::DOMAINNAME,
            0x04 => Self::IPv6,
            _ => Self::UNKOWN,
        }
    }
}
impl From<&SocketAddr> for ATYP {
    fn from(value: &SocketAddr) -> Self {
        if value.is_ipv6() {
            Self::IPv6
        } else {
            Self::IPv4
        }
    }
}
