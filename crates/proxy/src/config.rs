use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    /// 代理监听的 host
    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,

    /// 代理监听的 port
    #[arg(long, default_value_t = 7878)]
    pub port: u16,

    /// socks5 代理服务器的鉴权用户名，可传入None，禁用鉴权
    #[arg(long)]
    pub username: Option<String>,

    /// socks5 代理服务器的鉴权密码，可传入None，禁用鉴权
    #[arg(long)]
    pub password: Option<String>,
}

impl Config {
    /// 判断此配置是否启用了鉴权
    pub fn has_auth(&self) -> bool {
        self.username.is_some() && self.password.is_some()
    }
}
