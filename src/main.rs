use nucleus_http::{
    request::Request,
    routes::{BoxedFuture, Route},
    virtual_host::VirtualHost,
    Server,
};
use tokio;

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let listener_ip = "0.0.0.0:7878";
    println!("Listening on {listener_ip}");
    let localhost_vhost = VirtualHost::new(
        "localhost",
        "0.0.0.0:7878",
        "/Users/prestongarrisoniii/dev/source/nucleus-http/",
    );

    let mut server = Server::bind(listener_ip).await?;
    server.add_virtual_host(localhost_vhost).await;
    server
        .add_route(Route::get("/locals", Box::new(base_get)))
        .await;
    server.add_route(Route::get_static("/", "index.html")).await;

    server.serve().await.unwrap();
    return Ok(());
}

fn base_get(_req: &Request) -> BoxedFuture<String> {
    Box::pin(async move { "Hello From Rust Routes!".to_string() })
}

