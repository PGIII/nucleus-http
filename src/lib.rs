pub mod cookies;
pub mod http;
pub mod methods;
pub mod request;
pub mod response;
pub mod routes;
pub mod state;
pub mod thread_pool;
pub mod utils;
pub mod virtual_host;

use anyhow::Context;
use bytes::{BufMut, BytesMut};

use response::Response;
use routes::Router;
use std::{path::Path, sync::Arc, vec};
use tokio::{
    self,
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpListener,
    select,
    signal::unix::{signal, SignalKind},
    sync::RwLock,
    task::JoinHandle,
};
use tokio_rustls::{
    rustls::{self, Certificate, PrivateKey},
    TlsAcceptor,
};
use tokio_util::sync::CancellationToken;

pub struct Server<S> {
    listener: TcpListener,
    acceptor: Option<TlsAcceptor>,
    router: Arc<RwLock<Router<S>>>,
    virtual_hosts: Arc<RwLock<Vec<virtual_host::VirtualHost>>>,
    cancel: CancellationToken,
}

trait Stream: AsyncWrite + AsyncRead + Unpin + Send + Sync {}

// Auto Implement Stream for all types that implent asyncRead + asyncWrite
impl<T> Stream for T where T: AsyncRead + AsyncWrite + Unpin + Send + Sync {}

pub struct Connection {
    stream: Box<dyn Stream>,
    client_ip: std::net::SocketAddr,
    virtual_hosts: Arc<RwLock<Vec<virtual_host::VirtualHost>>>,
}

impl Connection {
    #[tracing::instrument(level = "debug", skip(self))]
    pub fn virtual_hosts(&self) -> Arc<RwLock<Vec<virtual_host::VirtualHost>>> {
        self.virtual_hosts.clone()
    }

    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn write_all(&mut self, src: &[u8]) -> tokio::io::Result<()> {
        self.stream.write_all(src).await?;
        Ok(())
    }

    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn write_response(&mut self, response: Response) -> tokio::io::Result<()> {
        let response_buffer = response.to_send_buffer();
        self.write_all(&response_buffer).await?;
        Ok(())
    }
}

impl<S> Server<S>
where
    S: Clone + Send + Sync + 'static,
{
    #[tracing::instrument(level = "debug", skip(router))]
    pub async fn bind(ip: &str, router: Router<S>) -> Result<Self, tokio::io::Error> {
        let listener = tokio::net::TcpListener::bind(ip).await?;
        Ok(Server {
            listener,
            router: Arc::new(RwLock::new(router)),
            virtual_hosts: Arc::new(RwLock::new(vec![])),
            acceptor: None,
            cancel: CancellationToken::new(),
        })
    }

    #[tracing::instrument(level = "debug", skip(router))]
    pub async fn bind_tls(
        ip: &str,
        cert: &Path,
        key: &Path,
        router: Router<S>,
    ) -> Result<Self, anyhow::Error> {
        let files = vec![cert, key];
        let context = format!("Opening: {:#?}, {:#?}", cert, key);
        let (mut keys, certs) = load_keys_and_certs(&files).context(context)?;
        let config = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, keys.remove(0))
            .context("Loading Certs")?;
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let listener = tokio::net::TcpListener::bind(ip)
            .await
            .context("binding tls")?;
        Ok(Server {
            listener,
            router: Arc::new(RwLock::new(router)),
            virtual_hosts: Arc::new(RwLock::new(vec![])),
            acceptor: Some(acceptor),
            cancel: CancellationToken::new(),
        })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    pub fn virtual_hosts(&self) -> Arc<RwLock<Vec<virtual_host::VirtualHost>>> {
        self.virtual_hosts.clone()
    }

    #[tracing::instrument(level = "debug", skip(self, virtual_host))]
    pub async fn add_virtual_host(&mut self, virtual_host: virtual_host::VirtualHost) {
        let virtual_hosts = self.virtual_hosts();
        let mut locked = virtual_hosts.write().await;
        locked.push(virtual_host);
    }

    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn accept(&self) -> tokio::io::Result<Connection> {
        let (stream, client_ip) = self.listener.accept().await?;
        if let Some(acceptor) = &self.acceptor {
            let acceptor = acceptor.clone();
            match acceptor.accept(stream).await {
                Ok(s) => Ok(Connection {
                    client_ip,
                    stream: Box::new(tokio_rustls::TlsStream::Server(s)),
                    virtual_hosts: self.virtual_hosts(),
                }),
                Err(_) => Err(tokio::io::Error::new(
                    tokio::io::ErrorKind::Other,
                    "Error Accepting TLS Stream",
                )),
            }
        } else {
            Ok(Connection {
                client_ip,
                stream: Box::new(stream),
                virtual_hosts: self.virtual_hosts(),
            })
        }
    }

    #[tracing::instrument(level = "debug", skip(self, connection))]
    fn serve_connection(&self, mut connection: Connection) -> JoinHandle<()> {
        let router = self.router.clone();
        let token = self.cancel.clone();
        let read_loop = async move {
            let mut request_bytes = BytesMut::with_capacity(1024);
            loop {
                let mut buffer = vec![0; 1024]; //Vector to avoid buffer on stack
                match connection.stream.read(&mut buffer).await {
                    Ok(0) => {
                        tracing::debug!("Connection Terminated by client");
                        break;
                    }
                    Ok(n) => {
                        //got some bytes append them and see if we need to do any proccessing
                        for b in buffer.iter().take(n) {
                            request_bytes.put_u8(*b);
                        }
                        let request_result =
                            request::Request::from_bytes(request_bytes.clone().into());
                        match request_result {
                            Ok(r) => {
                                tracing::info!(
                                    "{} Request for: {} from {}",
                                    r.method(),
                                    r.path(),
                                    connection.client_ip
                                );
                                let router_locked = router.read().await;
                                let response =
                                    router_locked.route(&r, connection.virtual_hosts()).await;
                                if let Err(error) = connection.write_response(response).await {
                                    // not clearing string here so we can try
                                    // again, otherwise might be terminated
                                    // connection which will be handled
                                    tracing::error!(
                                        "Error Writing response: {}",
                                        error.to_string()
                                    );
                                } else {
                                    //clear buffer
                                    request_bytes.clear();
                                }
                                drop(r);
                            }
                            Err(e) => match e {
                                request::Error::InvalidString
                                | request::Error::MissingBlankLine => {}
                                request::Error::WaitingOnBody(pb) => {
                                    if let Some(bytes_left) = pb {
                                        let free_bytes =
                                            request_bytes.capacity() - request_bytes.len();
                                        if free_bytes < bytes_left {
                                            // we know body size preallocate for it
                                            request_bytes.reserve(bytes_left - free_bytes);
                                        }
                                    }
                                }
                                _ => {
                                    let error_res = format!("400 bad request: {}", e);
                                    tracing::debug!("{}", error_res);
                                    let response = Response::error(
                                        http::StatusCode::ErrBadRequest,
                                        error_res.into(),
                                    );
                                    if let Err(err) = connection.write_response(response).await {
                                        tracing::error!("Error Writing Data: {}", err.to_string());
                                    }
                                }
                            },
                        }
                    }
                    Err(err) => {
                        tracing::error!("Socket read error: {}", err.to_string());
                        break;
                    }
                }
            }
        };

        tokio::spawn(async move {
            select! {
                _ = read_loop => {
                }
                _ = token.cancelled() => {
                    tracing::debug!("shutting down listen thread");
                }
            }
        })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn serve(&self) -> tokio::io::Result<()> {
        let accept_loop = async move {
            loop {
                let accept_attempt = self.accept().await;
                match accept_attempt {
                    Ok(connection) => {
                        tracing::info!("Accepted Connection From {}", connection.client_ip);
                        self.serve_connection(connection);
                    }
                    Err(e) => {
                        tracing::error!("Error Accepting Connection: {}", e.to_string());
                    }
                }
            }
        };

        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        select! {
            _ = accept_loop => {
                tracing::info!("shutting down due to acceptor exit");
                Ok(())
            }
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Received CTRL C shutting down");
                self.cancel.cancel();
                Ok(())
            }
            _ = sigterm.recv() => {
                tracing::info!("Received SigTerm shutting down");
                self.cancel.cancel();
                Ok(())
            }
        }
    }
}
fn load_keys_and_certs(paths: &Vec<&Path>) -> std::io::Result<(Vec<PrivateKey>, Vec<Certificate>)> {
    let mut keys = vec![];
    let mut certs = vec![];
    for path in paths {
        let items =
            rustls_pemfile::read_all(&mut std::io::BufReader::new(std::fs::File::open(path)?))?;
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
