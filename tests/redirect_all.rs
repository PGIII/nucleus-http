mod common;
use get_port::tcp::TcpPort;
use get_port::{Ops, Range};
use log;
use nucleus_http::{
    routes::{Route, Router},
    virtual_host::VirtualHost,
    Server,
};
use std::format;

#[tokio::test]
async fn redirect_all() {
    pretty_env_logger::init();
    let tcp_port = TcpPort::in_range(
        "127.0.0.1",
        Range {
            min: 6000,
            max: 8000,
        },
    )
    .unwrap();
    let listener_ip = format!("0.0.0.0:{}", tcp_port);
    log::info!("Listening on {listener_ip}");
    let localhost_vhost = VirtualHost::new("localhost", &listener_ip, "./");

    let mut router = Router::new(());
    router.add_route(Route::redirect_all("/index.html")).await;
    let mut server = Server::bind(&listener_ip, router).await.unwrap();
    server.add_virtual_host(localhost_vhost).await;
    tokio::spawn(async move { server.serve().await.expect("Server Shutdown") });

    let url = format!("http://localhost:{}/", tcp_port);
    let expected = format!("http://localhost:{}/index.html", tcp_port);
    let client_builder = reqwest::ClientBuilder::new();
    let client = client_builder.build().unwrap();
    let res = client.get(&url).send().await.unwrap();
    assert_eq!(expected, res.url().to_string());
}
