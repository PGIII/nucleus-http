use log;
use nucleus_http::{
    request::Request,
    routes::{BoxedFuture, Route},
    virtual_host::VirtualHost,
    Server,
};
use pretty_env_logger;
use tokio;

use argh::FromArgs;
use rustls_pemfile::read_all;
use std::fs::File;
use std::io::{self, BufReader};
use std::net::ToSocketAddrs;
use std::path::{Path, PathBuf};

/// Tokio Rustls server example
#[derive(FromArgs)]
struct Options {
    /// bind addr
    #[argh(positional)]
    addr: String,

    /// cert file
    #[argh(option, short = 'c')]
    cert: PathBuf,

    /// key file
    #[argh(option, short = 'k')]
    key: PathBuf,
}

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    pretty_env_logger::init();
    let options: Options = argh::from_env();

    let addr = options
        .addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::from(io::ErrorKind::AddrNotAvailable))?;
    let listener_ip = &addr.to_string();
    log::info!("Listening on {listener_ip}");
    let localhost_vhost = VirtualHost::new(
        "localhost",
        listener_ip,
        "/Users/prestongarrisoniii/dev/source/nucleus-http/",
    );

    let mut server = Server::bind_tls(listener_ip, &options.cert, &options.key).await?;
    server.add_virtual_host(localhost_vhost).await;
    server
        .add_route(Route::get_async("/async", Box::new(async_get)))
        .await;
    server.add_route(Route::get("/sync", get)).await;
    server.add_route(Route::get_static("/", "index.html")).await;

    server.serve().await.unwrap();
    Ok(())
}

fn async_get(_req: &Request) -> BoxedFuture<String> {
    Box::pin(async move { "Hello From Rust Routes!".to_string() })
}

fn get(_req: &Request) -> String {
    "Hello From Sync Func".to_string()
}
