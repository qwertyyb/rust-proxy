开发测试方式

使用brook进行开发测试

```bash
brew install brook

# 启动一个本地的dns server
brook --log console dnsserver --listen 127.0.0.1:5354 --dns "192.168.31.1:53"

# 通过socks代理发起dns查询
brook testsocks5 --socks5 127.0.0.1:7878 --domain "baidu.com" --dns "127.0.0.1:5354" -a 39.156.66.10

```