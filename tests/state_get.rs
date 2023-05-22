mod common;
use common::*;

#[tokio::test]
async fn state_dynamic_get() {
    let tcp_port = launch_dev_server().await;
    let client_builder = reqwest::ClientBuilder::new();
    let client = client_builder.build().unwrap();

    let url = format!("http://localhost:{}/state/bob", tcp_port);
    let res = client.get(&url).send().await.unwrap();
    let body = "HI /state/bob and Bye, Viewed: 0";
    assert_eq!(url, res.url().to_string());
    assert_eq!(reqwest::StatusCode::from_u16(200).unwrap(), res.status());
    assert_eq!(body, res.text().await.unwrap());

    let url = format!("http://localhost:{}/state/jerry", tcp_port);
    let res = client.get(&url).send().await.unwrap();
    let body = "HI /state/jerry and Bye, Viewed: 1";
    assert_eq!(url, res.url().to_string());
    assert_eq!(reqwest::StatusCode::from_u16(200).unwrap(), res.status());
    assert_eq!(body, res.text().await.unwrap());
}
