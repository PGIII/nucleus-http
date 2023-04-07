use std::sync::Arc;
use http::Version;
use response::Response;
use tokio::{
    self,
    io::{AsyncBufReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::RwLock,
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
    routes: Arc<RwLock<routes::Routes>>,
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
        println!("Writing: {response_str}");
        self.write_all(&response_str.into_bytes()).await?;
        Ok(())
    }
}

impl Server {
    pub async fn add_route(&mut self, route: routes::Route) {
        let mut routes_locked = self.routes.write().await;
        routes_locked.add(route);
    }

    pub async fn bind(ip: &str) -> Result<Server, tokio::io::Error> {
        let listener = tokio::net::TcpListener::bind(ip).await?;
        Ok(Server {
            listener,
            routes: Arc::new(RwLock::new(routes::Routes::new())),
        })
    }

    pub fn routes(&self) -> Arc<RwLock<routes::Routes>> {
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
                        let response = routes.read().await.run(&r).await;
                        connection.write_response(response).await.unwrap();
                    }
                    Err(e) => {
                        dbg!(e);
                        match e {
                            _ => {
                                let response = "HTTP/1.1 400 Error\r\n";
                                connection.write_all(response.as_bytes()).await.unwrap();
                            }
                        }
                    }
                }
            });
        }
    }
}
