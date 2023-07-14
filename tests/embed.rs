mod common;
use get_port::tcp::TcpPort;
use get_port::{Ops, Range};
use nucleus_http::{
    routes::{Route, Router},
    Server,
};
use std::format;
use std::path::PathBuf;

#[tokio::test]
async fn embed() {
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

    let mut router = Router::new(());
    router
        .add_route(nucleus_http::embed_route!("/", "../index.html"))
        .await;
    let server = Server::bind(&listener_ip, router, "./").await.unwrap();
    tokio::spawn(async move { server.serve().await.expect("Server Shutdown") });

    let url = format!("http://localhost:{}/", tcp_port);
    let index_string = include_str!("../index.html");
    let client_builder = reqwest::ClientBuilder::new();
    let client = client_builder.build().unwrap();
    let res = client.get(&url).send().await.unwrap();
    assert_eq!(url, res.url().to_string());
    assert_eq!(reqwest::StatusCode::from_u16(200).unwrap(), res.status());
    assert_eq!(index_string, res.text().await.unwrap());
}
