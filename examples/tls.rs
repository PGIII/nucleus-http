use argh::FromArgs;
use nucleus_http::{
    request::Request,
    routes::{Route, Router},
    Server,
};
use std::io;
use std::net::ToSocketAddrs;
use std::path::PathBuf;

/// Tokio Rustls server example
#[derive(FromArgs)]
struct Options {
    /// tls bind addr
    #[argh(positional)]
    tls_addr: String,

    /// http bind addr
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
async fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();
    let options: Options = argh::from_env();

    let addr = options
        .addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::from(io::ErrorKind::AddrNotAvailable))?;

    let tls_addr = options
        .tls_addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::from(io::ErrorKind::AddrNotAvailable))?;

    let listener_ip = &tls_addr.to_string();
    log::info!("Listening on {listener_ip}");

    let mut router = Router::new(());
    router
        .add_route(Route::get("/async", Box::new(async_get)))
        .await;
    router.add_route(Route::get("/sync", Box::new(get))).await;
    router.add_route(Route::get_static("/", "index.html")).await;

    let server = Server::bind_tls(listener_ip, &options.cert, &options.key, router, "./").await?;
    tokio::spawn(launch_http(addr, tls_addr));
    server.serve().await.unwrap();
    Ok(())
}

async fn async_get(_: (), _: Request) -> Result<String, String> {
    Ok("Hello From Rust Routes!".to_string())
}

async fn get(_: (), _: Request) -> Result<String, String> {
    Ok("Hello From Sync Func".to_string())
}

async fn launch_http(
    addr: std::net::SocketAddr,
    tls_addr: std::net::SocketAddr,
) -> tokio::io::Result<()> {
    let listener_ip = addr;
    log::info!("Redirecting all on {addr} to {tls_addr}");
    let mut router = Router::new(());
    router
        .add_route(Route::redirect_all(&format!("https://{tls_addr}/")))
        .await;
    let server = Server::bind(&listener_ip.to_string(), router, "./").await?;
    server.serve().await?;
    Ok(())
}
