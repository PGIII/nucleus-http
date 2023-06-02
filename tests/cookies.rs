mod common;
use common::*;

#[tokio::test]
async fn cookies() {
    let tcp_port = launch_dev_server().await;
    let client_builder = reqwest::ClientBuilder::new().cookie_store(true);
    let client = client_builder.build().unwrap();

    let url = format!("http://localhost:{}/get_cookie", tcp_port);
    let res = client.get(&url).send().await.unwrap();
    assert_eq!(url, res.url().to_string(), "Invalid URL");
    assert_eq!(reqwest::StatusCode::from_u16(200).unwrap(), res.status());
    let cookie = res.cookies().next().expect("No Cookies");
    assert_eq!(cookie.name(), "flavor");

    let url2 = format!("http://localhost:{}/whats_my_cookie", tcp_port);
    let res2 = client.get(&url2).send().await.unwrap();
    assert_eq!(url2, res2.url().to_string(), "Invalid URL");
    assert_eq!(reqwest::StatusCode::from_u16(200).unwrap(), res2.status());
    assert_eq!(
        "You Have A chocolate chip Cookie",
        res2.text().await.expect("Error Receiving Body")
    );
}
