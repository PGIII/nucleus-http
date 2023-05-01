use acme_lib::create_p384_key;
use acme_lib::persist::FilePersist;
use acme_lib::{Directory, DirectoryUrl};
use argh::FromArgs;
use log;
use nucleus_http::{
    request::Request,
    routes::{BoxedFuture, Route},
    virtual_host::VirtualHost,
    Server,
};
use pretty_env_logger;
use std::io;
use std::net::ToSocketAddrs;
use std::sync::RwLock;
use tokio;

/// Tokio Rustls server example
#[derive(FromArgs)]
struct Options {
    /// tls bind addr
    #[argh(positional)]
    tls_addr: String,

    /// http bind addr
    #[argh(positional)]
    addr: String,
}

static CHALLENGE_RESPONSE: RwLock<String>  = RwLock::new(String::new()); 

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
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
    let localhost_vhost = VirtualHost::new("localhost", listener_ip, "./");

    /*
    let mut server = Server::bind_tls(listener_ip, &options.cert, &options.key).await?;
    server.add_virtual_host(localhost_vhost).await;
    server
        .add_route(Route::get_async("/async", Box::new(async_get)))
        .await;
    server.add_route(Route::get("/sync", get)).await;
    server.add_route(Route::get_static("/", "index.html")).await;
    */
    tokio::spawn(launch_http(addr, tls_addr));
    request_cert().unwrap();
    //server.serve().await.unwrap();
    Ok(())
}

async fn launch_http(
    addr: std::net::SocketAddr,
    tls_addr: std::net::SocketAddr,
) -> tokio::io::Result<()> {
    let listener_ip = addr;
    let localhost_vhost = VirtualHost::new("localhost", &listener_ip.to_string(), "./");
    log::info!("Redirecting all on {addr} to {tls_addr}");
    let mut server = Server::bind(&listener_ip.to_string()).await?;
    server.add_virtual_host(localhost_vhost).await;
    server
        .add_route(Route::redirect_all(&format!("https://{tls_addr}/")))
    .await;
    server.add_route(Route::get("/.well-known/*", challenge_serve)).await;
    server.serve().await?;
    Ok(())
}

fn get(_req: &Request) -> String {
    "Hello From Sync Func".to_string()
}

fn challenge_serve(_: &Request) -> String {
    CHALLENGE_RESPONSE.read().unwrap().clone()
}

fn request_cert() -> Result<(), acme_lib::Error> {
    // Use DirectoryUrl::LetsEncrypStaging for dev/testing.
    let url = DirectoryUrl::LetsEncryptStaging;

    // Save/load keys and certificates to current dir.
    let persist = FilePersist::new(".");

    // Create a directory entrypoint.
    let dir = Directory::from_url(persist, url)?;

    // Reads the private account key from persistence, or
    // creates a new one before accessing the API to establish
    // that it's there.
    let acc = dir.account("preston.garrison3@gmail.com")?;

    // Order a new TLS certificate for a domain.
    let mut ord_new = acc.new_order("acme.pg3.dev", &["acme.preston3.com"])?;

    // If the ownership of the domain(s) have already been
    // authorized in a previous order, you might be able to
    // skip validation. The ACME API provider decides.
    let ord_csr = loop {
        // are we done?
        if let Some(ord_csr) = ord_new.confirm_validations() {
            break ord_csr;
        }

        // Get the possible authorizations (for a single domain
        // this will only be one element).
        let auths = ord_new.authorizations()?;

        // For HTTP, the challenge is a text file that needs to
        // be placed in your web server's root:
        //
        // /var/www/.well-known/acme-challenge/<token>
        //
        // The important thing is that it's accessible over the
        // web for the domain(s) you are trying to get a
        // certificate for:
        //
        // http://mydomain.io/.well-known/acme-challenge/<token>
        let chall = auths[0].http_challenge();

        // The token is the filename.
        let token = chall.http_token();
        let path = format!(".well-known/acme-challenge/{}", token);

        // The proof is the contents of the file
        let proof = chall.http_proof();

        // Here you must do "something" to place
        // the file/contents in the correct place.
        // update_my_web_server(&path, &proof);
        let mut locked = CHALLENGE_RESPONSE.write().unwrap();
        *locked = proof.clone();

        // After the file is accessible from the web, the calls
        // this to tell the ACME API to start checking the
        // existence of the proof.
        //
        // The order at ACME will change status to either
        // confirm ownership of the domain, or fail due to the
        // not finding the proof. To see the change, we poll
        // the API with 5000 milliseconds wait between.
        chall.validate(5000)?;

        // Update the state against the ACME API.
        ord_new.refresh()?;
    };

    // Ownership is proven. Create a private key for
    // the certificate. These are provided for convenience, you
    // can provide your own keypair instead if you want.
    let pkey_pri = create_p384_key();

    // Submit the CSR. This causes the ACME provider to enter a
    // state of "processing" that must be polled until the
    // certificate is either issued or rejected. Again we poll
    // for the status change.
    let ord_cert = ord_csr.finalize_pkey(pkey_pri, 5000)?;

    // Now download the certificate. Also stores the cert in
    // the persistence.
    let cert = ord_cert.download_and_save_cert()?;

    Ok(())
}
fn async_get(_req: &Request) -> BoxedFuture<String> {
    Box::pin(async move { "Hello From Rust Routes!".to_string() })
}


