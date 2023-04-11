use nucleus_http::{
    request::Request, routes::Route, virtual_host::VirtualHost, Server,
};
use tokio;

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let listener_ip = "0.0.0.0:7878";
    println!("Listening on {listener_ip}");
    let localhost_vhost = VirtualHost::new("localhost", "0.0.0.0:7878", "/Users/prestongarrisoniii/dev/source/nucleus-http/");

    let mut server = Server::bind(listener_ip).await?;
    server.add_virtual_host(localhost_vhost).await;
    server
        .add_route(Route::get(
            "/locals".to_owned(),
            base_get,
        ))
        .await;

    tokio::spawn(run_server2());

    server.serve().await.unwrap();
    return Ok(());
}

async fn run_server2() {
    let mut server2 = Server::bind("0.0.0.0:7777").await.unwrap();
    server2
        .add_route(Route::get_static(
            "/".to_owned(),
            "/Users/prestongarrisoniii/dev/source/nucleus-http/index.html".to_string(),
        ))
        .await;
    server2.serve().await.unwrap();
}

fn base_get(_req: &Request) -> String {
    "Hello From Rust Routes!".to_string()
}
