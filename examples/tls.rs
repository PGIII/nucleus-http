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
use std::sync::Arc;
use tokio::io::{copy, sink, split, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::rustls::{self, Certificate, PrivateKey};
use tokio_rustls::TlsAcceptor;

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

    /// echo mode
    #[argh(switch, short = 'e')]
    echo_mode: bool,
}

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    pretty_env_logger::init();
    /*
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
    */
    let options: Options = argh::from_env();

    let addr = options
        .addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::from(io::ErrorKind::AddrNotAvailable))?;
    let files: Vec<&Path> = vec![&options.cert, &options.key];
    let (mut keys, certs) = load_keys_and_certs(&files)?;
    let flag_echo = options.echo_mode;

    let config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, keys.remove(0))
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
    let acceptor = TlsAcceptor::from(Arc::new(config));

    let listener = TcpListener::bind(&addr).await?;

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        let acceptor = acceptor.clone();

        let fut = async move {
            let mut stream = acceptor.accept(stream).await?;

            if flag_echo {
                let (mut reader, mut writer) = split(stream);
                let n = copy(&mut reader, &mut writer).await?;
                writer.flush().await?;
                println!("Echo: {} - {}", peer_addr, n);
            } else {
                let mut output = sink();
                stream
                    .write_all(
                        &b"HTTP/1.0 200 ok\r\n\
                            Connection: close\r\n\
                            Content-length: 12\r\n\
                            \r\n\
                            Hello world!"[..],
                    )
                    .await?;
                stream.shutdown().await?;
                copy(&mut stream, &mut output).await?;
                println!("Hello: {}", peer_addr);
            }

            Ok(()) as io::Result<()>
        };

        tokio::spawn(async move {
            if let Err(err) = fut.await {
                eprintln!("{:?}", err);
            }
        });
    }
}

fn async_get(_req: &Request) -> BoxedFuture<String> {
    Box::pin(async move { "Hello From Rust Routes!".to_string() })
}

fn get(_req: &Request) -> String {
    "Hello From Sync Func".to_string()
}

/// Load all keys and certs from list of files passed
fn load_keys_and_certs(paths: &Vec<&Path>) -> io::Result<(Vec<PrivateKey>, Vec<Certificate>)> {
    let mut keys = vec![];
    let mut certs = vec![];
    for path in paths {
        let items = read_all(&mut BufReader::new(File::open(path)?))?;
        for item in items {
            match item {
                rustls_pemfile::Item::RSAKey(key) => {
                    keys.push(PrivateKey(key));
                }
                rustls_pemfile::Item::ECKey(key) => {
                    keys.push(PrivateKey(key));
                }
                rustls_pemfile::Item::PKCS8Key(key) => {
                    keys.push(PrivateKey(key));
                }
                rustls_pemfile::Item::X509Certificate(cert) => {
                    certs.push(Certificate(cert));
                }
                _ => {}
            }
        }
    }
    Ok((keys, certs))
}
