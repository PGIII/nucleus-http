use http::MimeType;
use log;
use request::Request;
use response::Response;
use routes::{ResolveFunction, Routes};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    self, fs,
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpListener,
    sync::RwLock,
};
use tokio_rustls::{
    rustls::{self, Certificate, PrivateKey},
    TlsAcceptor,
};

pub mod http;
pub mod methods;
pub mod request;
pub mod response;
pub mod routes;
pub mod thread_pool;
pub mod virtual_host;

pub struct Server {
    listener: TcpListener,
    acceptor: Option<TlsAcceptor>,
    routes: Routes,
    virtual_hosts: Arc<RwLock<Vec<virtual_host::VirtualHost>>>,
}

trait Stream: AsyncWrite + AsyncRead + Unpin + Send + Sync {}

// Auto Implement Stream for all types that implent asyncRead + asyncWrite
impl<T> Stream for T where T: AsyncRead + AsyncWrite + Unpin + Send + Sync {}

pub struct Connection {
    stream: Box<dyn Stream>,
    client_ip: std::net::SocketAddr,
    routes: Routes,
    virtual_hosts: Arc<RwLock<Vec<virtual_host::VirtualHost>>>,
}

impl Connection {
    pub fn routes(&self) -> Routes {
        return self.routes.clone();
    }

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

impl Server {
    pub async fn add_route(&mut self, route: routes::Route) {
        let mut routes_locked = self.routes.write().await;
        routes_locked.push(route);
    }

    pub async fn bind(ip: &str) -> Result<Server, tokio::io::Error> {
        let listener = tokio::net::TcpListener::bind(ip).await?;
        Ok(Server {
            listener,
            routes: Arc::new(RwLock::new(vec![])),
            virtual_hosts: Arc::new(RwLock::new(vec![])),
            acceptor: None,
        })
    }

    pub async fn bind_tls(ip: &str, cert: &Path, key: &Path) -> Result<Server, tokio::io::Error> {
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
            routes: Arc::new(RwLock::new(vec![])),
            virtual_hosts: Arc::new(RwLock::new(vec![])),
            acceptor: Some(acceptor),
        })
    }

    pub fn routes(&self) -> Routes {
        return Arc::clone(&self.routes);
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
                    routes: self.routes(),
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
                routes: self.routes(),
                virtual_hosts: self.virtual_hosts(),
            });
        }
    }

    pub async fn serve(&self) -> tokio::io::Result<()> {
        loop {
            let accept_attempt = self.accept().await;
            match accept_attempt {
                Ok(mut connection) => {
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
                                                let response = Self::route(&r, &connection).await;
                                                connection.write_response(response).await.unwrap();
                                                request_str.clear();
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

    async fn run_sync_func(request: Request, func: ResolveFunction) -> Response {
        let blocking = tokio::task::spawn_blocking(move || {
            let result = func(&request);
            return result;
        })
        .await;
        //FIXME: return error response intead of unwap
        return Response::from(blocking.unwrap());
    }

    async fn route(request: &Request, connection: &Connection) -> Response {
        log::info!("{} Request for: {}", request.method(), request.path());
        let routes = connection.routes();
        let routes_locked = routes.read().await;

        for route in &*routes_locked {
            if Self::routes_request_match(request, &route) {
                match route.resolver() {
                    routes::RouteResolver::AsyncFunction(func) => {
                        let func_return = func(&request).await;
                        return Response::from(func_return);
                    }
                    routes::RouteResolver::Function(func) => {
                        return Self::run_sync_func(request.to_owned(), func.to_owned()).await;
                    }
                    routes::RouteResolver::Static { file_path } => {
                        if let Some(host_dir) = Self::get_vhost_dir(request, connection).await {
                            let path = host_dir.join(file_path);
                            return Self::get_file(path).await;
                        }
                    }
                }
            }
        }
        if let Some(host_dir) = Self::get_vhost_dir(request, connection).await {
            let mut file_path = PathBuf::from(request.path());
            if file_path.is_absolute() {
                if let Ok(path) = file_path.strip_prefix("/") {
                    file_path = path.to_path_buf();
                } else {
                    return Response::error(http::StatusCode::ErrNotFound, "File Not Found".into());
                }
            }
            let final_path = host_dir.join(file_path);
            return Self::get_file(final_path).await;
        }

        //no route try static serve
        let response = Response::error(http::StatusCode::ErrNotFound, "File Not Found".into());
        return response;
    }

    async fn get_vhost_dir(request: &Request, connection: &Connection) -> Option<PathBuf> {
        for vhost in &*connection.virtual_hosts().read().await {
            if vhost.hostname() == request.hostname() {
                return Some(vhost.root_dir().to_path_buf());
            }
        }
        return None;
    }

    async fn get_file(path: PathBuf) -> Response {
        match fs::read(&path).await {
            Ok(contents) => {
                let mime: MimeType = path.into();
                let response = Response::new(http::StatusCode::OK, contents, mime);
                return response;
            }
            Err(err) => match err.kind() {
                std::io::ErrorKind::PermissionDenied => {
                    let response =
                        Response::error(http::StatusCode::ErrForbidden, "Permission Denied".into());
                    return response;
                }
                std::io::ErrorKind::NotFound | _ => {
                    let response = Response::error(
                        http::StatusCode::ErrNotFound,
                        "Static File Not Found".into(),
                    );
                    return response;
                }
            },
        }
    }

    fn routes_request_match(request: &Request, route: &routes::Route) -> bool {
        let path_match = request.path() == route.path();
        let methods_match = request.method() == route.method();
        return methods_match && path_match;
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
