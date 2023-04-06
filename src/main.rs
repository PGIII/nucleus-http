use rust_web_server::serve;
use tokio;

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let listener_ip = "0.0.0.0:7878";
    println!("Listening on {listener_ip}");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:7878").await?;
    let result = serve(listener).await;
    return result;
}
