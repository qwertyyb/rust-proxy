# rust-proxy

使用 rust 开发的符合 RFC 1928 和 RFC 1929 规范的 socks5 代理服务器

相关规范如下:

1. [socks5 代理协议规范 RFC 1928](https://www.ietf.org/rfc/rfc1928.txt)
2. [socks5 账号密码鉴权规范 RFC 1929](https://www.ietf.org/rfc/rfc1929.txt)

## 特性

1. 支持 socks5 代理 TCP 和 UDP
2. UDP 穿透 Full Clone

## 开发

开发测试方式

### 1. TCP 测试

TCP 相对来说比较容易测试，使用curl即可

```bash
# 无密码校验
curl https://www.baidu.com -x socks5://127.0.0.1:7878 -v


# 有密码校验
curl https://www.baidu.com -x socks5://hello:world@127.0.0.1:7878 -v
```

### 2. UDP 测试

UDP测试就比较难了，推荐安装 brook 进行 UDP 的开发测试

```bash
brew install brook

# 启动一个本地的dns server
brook --log console dnsserver --listen 127.0.0.1:5354 --dns "192.168.31.1:53"

# 通过socks代理发起dns查询
brook testsocks5 --socks5 127.0.0.1:7878 --domain "baidu.com" --dns "127.0.0.1:5354" -a 39.156.66.10

```

## 已知问题

1. UDP 穿透时新增的线程I/O阻塞无法停止

表现为每有一个新的 UDP 流量过来时，会启动两个交换数据的线程（client -> socks5 -> server，server -> socks5 -> client），这两个线程不会停止。

这个主要是因为两个线程都被I/O阻塞了（UdpSocket::read），而rust官方目前没有提供一种官方的方式来异步交换数据，也没有提供一种方法来终止掉一个线程。



