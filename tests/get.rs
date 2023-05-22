mod common;
use common::*;

#[tokio::test]
async fn file_get() {
    let tcp_port = launch_dev_server().await;
    let url = format!("http://localhost:{}/", tcp_port);
    let client_builder = reqwest::ClientBuilder::new();
    let client = client_builder.build().unwrap();
    let res = client.get(&url).send().await.unwrap();
    dbg!(&res);
    let index_string = include_str!("../index.html");
    assert_eq!(url, res.url().to_string());
    assert_eq!(reqwest::StatusCode::from_u16(200).unwrap(), res.status());
    assert_eq!(index_string, res.text().await.unwrap());
}
