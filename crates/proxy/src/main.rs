use rust_proxy::launch_from_cli;

#[tokio::main]
async fn main() {
    // 注意，env_logger 必须尽可能早的初始化
    env_logger::init();
    launch_from_cli().await;
}
