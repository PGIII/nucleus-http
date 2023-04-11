use request::Request;
use response::Response;
use std::sync::Arc;
use tokio::{
    self,
    io::{AsyncBufReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::RwLock,
    fs
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
    routes: Arc<RwLock<Vec<routes::Route>>>,
}

pub struct Connection {
    stream: TcpStream,
}

impl Connection {
    pub async fn write_all(&mut self, src: &[u8]) -> tokio::io::Result<()> {
        self.stream.write_all(src).await?;
        Ok(())
    }

    pub async fn write_response(&mut self, response: Response) -> tokio::io::Result<()> {
        let response_str: String = response.into();
        self.write_all(&response_str.into_bytes()).await?;
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
        })
    }

    pub fn routes(&self) -> Arc<RwLock<Vec<routes::Route>>> {
        return Arc::clone(&self.routes);
    }

    pub async fn accept(&self) -> tokio::io::Result<Connection> {
        let (stream, _) = self.listener.accept().await?;
        Ok(Connection { stream })
    }

    pub async fn serve(&self) -> tokio::io::Result<()> {
        loop {
            let routes = self.routes();
            let mut connection = self.accept().await?;

            tokio::spawn(async move {
                let mut buf_reader = tokio::io::BufReader::new(&mut connection.stream);
                let mut request_str = "".to_owned();
                loop {
                    let mut line: String = "".to_string();
                    let result = buf_reader.read_line(&mut line).await;
                    match result {
                        Ok(_) => {
                            request_str += &line;
                            if &line == "\r\n" {
                                break;
                            }
                        }
                        Err(_) => {
                            break;
                        }
                    }
                }
                let request_result = request::Request::from_string(request_str);
                match request_result {
                    Ok(r) => {
                        let response = Self::route(&r, routes).await;
                        connection.write_response(response).await.unwrap();
                    }
                    Err(e) => {
                        dbg!(e);
                        match e {
                            _ => {
                                let response = Response::error(
                                    http::StatusCode::ErrBadRequest,
                                    "400 Bad Request".to_string(),
                                );
                                connection.write_response(response).await.unwrap();
                            }
                        }
                    }
                }
            });
        }
    }

    async fn route(request: &Request, routes: Arc<RwLock<Vec<routes::Route>>>) -> Response {
        let routes_locked = routes.read().await; 
        for route in &*routes_locked {
            if Self::routes_request_match(request, &route) {
                match route.resolver() {
                    routes::RouteResolver::Static { file_path } => {
                        let response;
                        if let Ok(contents) = fs::read_to_string(file_path).await {
                            response = Response::from(contents);
                        } else {
                            response = Response::error(
                                http::StatusCode::ErrNotFound,
                                "File Not Found".to_string(),
                            );
                        }
                        return response;
                    }
                    routes::RouteResolver::Function(func) => {
                        let func_return = func(&request);
                        return Response::from(func_return);
                    }
                }
            }
        }
        let response = Response::error(http::StatusCode::ErrNotFound, "File Not Found".to_string());
        return response;
    }

    fn routes_request_match(request: &Request, route: &routes::Route) -> bool {
        let path_match = request.path() == route.path();
        let host_match;
        let methods_match = request.method() == route.method();
        if let Some(vhost) = route.vhost() {
            host_match = request.hostname() == vhost.hostname();
        } else {
            host_match = true;
        }
        return methods_match && path_match && host_match;
    }
}
