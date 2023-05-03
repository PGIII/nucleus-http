pub mod http;
pub mod methods;
pub mod request;
pub mod response;
pub mod routes;
pub mod state;
pub mod thread_pool;
pub mod virtual_host;

use log;
use response::Response;
use routes::{Router, RequestResolver};
use std::{path::Path, sync::Arc};
use tokio::{
    self,
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpListener,
    sync::RwLock,
};
use tokio_rustls::{
    rustls::{self, Certificate, PrivateKey},
    TlsAcceptor,
};

pub struct Server<S, R> {
    listener: TcpListener,
    acceptor: Option<TlsAcceptor>,
    router: Arc<RwLock<Router<S, R>>>,
    virtual_hosts: Arc<RwLock<Vec<virtual_host::VirtualHost>>>,
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
    pub fn virtual_hosts(&self) -> Arc<RwLock<Vec<virtual_host::VirtualHost>>> {
        return self.virtual_hosts.clone();
    }

    pub async fn write_all(&mut self, src: &[u8]) -> tokio::io::Result<()> {
        self.stream.write_all(src).await?;
        Ok(())
    }

    pub async fn write_response(&mut self, response: Response) -> tokio::io::Result<()> {
        let response_buffer = response.to_send_buffer();
        self.write_all(&response_buffer).await?;
        Ok(())
    }
}

impl<S, R> Server<S, R> 
where
    S: Clone + Send + Sync +'static,
    R: RequestResolver<S> + Sync + Send + 'static + Copy
{
    pub async fn bind(ip: &str, router: Router<S, R>) -> Result<Self, tokio::io::Error> {
        let listener = tokio::net::TcpListener::bind(ip).await?;
        Ok(Server {
            listener,
            router: Arc::new(RwLock::new(router)),
            virtual_hosts: Arc::new(RwLock::new(vec![])),
            acceptor: None,
        })
    }

    pub async fn bind_tls(
        ip: &str,
        cert: &Path,
        key: &Path,
        router: Router<S, R>,
    ) -> Result<Self, tokio::io::Error> {
        let files = vec![cert, key];
        let (mut keys, certs) = load_keys_and_certs(&files)?;
        let config = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, keys.remove(0))
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let listener = tokio::net::TcpListener::bind(ip).await?;
        Ok(Server {
            listener,
            router: Arc::new(RwLock::new(router)),
            virtual_hosts: Arc::new(RwLock::new(vec![])),
            acceptor: Some(acceptor),
        })
    }

    pub fn virtual_hosts(&self) -> Arc<RwLock<Vec<virtual_host::VirtualHost>>> {
        return self.virtual_hosts.clone();
    }
    pub async fn add_virtual_host(&mut self, virtual_host: virtual_host::VirtualHost) {
        let virtual_hosts = self.virtual_hosts();
        let mut locked = virtual_hosts.write().await;
        locked.push(virtual_host);
    }

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
                Err(_) => {
                    return Err(tokio::io::Error::new(
                        tokio::io::ErrorKind::Other,
                        "Error Accepting TLS Stream",
                    ));
                }
            }
        } else {
            return Ok(Connection {
                client_ip,
                stream: Box::new(stream),
                virtual_hosts: self.virtual_hosts(),
            });
        }
    }

    pub async fn serve(&self) -> tokio::io::Result<()> {
        loop {
            let accept_attempt = self.accept().await;
            match accept_attempt {
                Ok(mut connection) => {
                    let router = self.router.clone();
                    log::info!("Accepted Connection From {}", connection.client_ip);
                    tokio::spawn(async move {
                        let mut request_str = String::new();
                        loop {
                            let mut buffer = vec![0; 1024]; //Vector to avoid buffer on stack
                            match connection.stream.read(&mut buffer).await {
                                Ok(0) => {
                                    log::debug!("Connection Terminated by client");
                                    break;
                                }
                                Ok(n) => {
                                    //got some bytes append them and see if we need to do any proccessing
                                    for i in 0..n {
                                        request_str.push(buffer[i] as char);
                                        let request_result =
                                            request::Request::from_string(request_str.clone());
                                        match request_result {
                                            Ok(r) => {
                                                let router_locked = router.read().await;
                                                let response =
                                                    router_locked.route(&r, &connection).await;
                                                if let Err(error) =
                                                    connection.write_response(response).await
                                                {
                                                    // not clearing string here so we can try
                                                    // again, otherwise might be terminated
                                                    // connection which will be handled
                                                    log::error!(
                                                        "Error Writing response: {}",
                                                        error.to_string()
                                                    );
                                                } else {
                                                    request_str.clear();
                                                }
                                            }
                                            Err(e) => match e {
                                                request::Error::InvalidString
                                                | request::Error::MissingBlankLine => {
                                                    //Parital response keep reading
                                                }
                                                _ => {
                                                    let error_res =
                                                        format!("400 bad request: {}", e);
                                                    let response = Response::error(
                                                        http::StatusCode::ErrBadRequest,
                                                        error_res.into(),
                                                    );
                                                    if let Err(err) =
                                                        connection.write_response(response).await
                                                    {
                                                        log::error!(
                                                            "Error Writing Data: {}",
                                                            err.to_string()
                                                        );
                                                    }
                                                }
                                            },
                                        }
                                    }
                                }
                                Err(err) => {
                                    log::error!("Socket read error: {}", err.to_string());
                                    break;
                                }
                            }
                        }
                    });
                }
                Err(e) => {
                    log::error!("Error Accepting Connection: {}", e.to_string());
                }
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
