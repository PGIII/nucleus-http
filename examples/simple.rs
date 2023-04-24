use nucleus_http::{
    request::Request,
    routes::{BoxedFuture, Route},
    virtual_host::VirtualHost,
    Server,
};
use tokio;
use pretty_env_logger;
use log;

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    pretty_env_logger::init();
    let listener_ip = "0.0.0.0:7878";
    log::info!("Listening on {listener_ip}");
    let localhost_vhost = VirtualHost::new(
        "localhost",
        "0.0.0.0:7878",
        "/Users/prestongarrisoniii/dev/source/nucleus-http/",
    );

    let mut server = Server::bind(listener_ip).await?;
    server.add_virtual_host(localhost_vhost).await;
    server
        .add_route(Route::get_async("/async", Box::new(async_get)))
        .await;
    server.add_route(Route::get("/sync", get)).await;
    server.add_route(Route::get_static("/", "index.html")).await;

    server.serve().await.unwrap();
    return Ok(());
}

fn async_get(_req: &Request) -> BoxedFuture<String> {
    Box::pin(async move { "Hello From Rust Routes!".to_string() })
}

fn get(_req: &Request) -> String {
    "Hello From Sync Func".to_string()
}
