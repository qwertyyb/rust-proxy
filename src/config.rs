use std::sync::OnceLock;

use clap::Parser;

static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,

    #[arg(short, long, default_value_t = 7878)]
    pub port: u16,

    /// socks5 proxy username
    #[arg(short, long)]
    pub username: Option<String>,

    /// socks5 proxy password
    #[arg(long)]
    pub password: Option<String>,
}

impl Config {
    pub fn global() -> &'static Self {
        CONFIG.get_or_init(|| Self::parse())
    }

    pub fn has_auth(&self) -> bool {
        return self.username.is_some() && self.password.is_some();
    }
}
