use std::sync::Arc;
use tokio::{
    self,
    io::{AsyncBufReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::RwLock,
};

pub mod methods;
pub mod request;
pub mod routes;
pub mod thread_pool;
pub mod virtual_host;

pub struct Server {
    listener: TcpListener,
    routes: Arc<RwLock<routes::Routes>>,
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

    pub async fn serve(&self) -> tokio::io::Result<()> {
        loop {
            let routes = self.routes();
            let (mut stream, _) = self.listener.accept().await?;

            tokio::spawn(async move {
                let mut buf_reader = tokio::io::BufReader::new(&mut stream);
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
                        stream.write_all(response.as_bytes()).await.unwrap();
                    }
                    Err(e) => {
                        dbg!(e);
                        match e {
                            _ => {
                                let response = "HTTP/1.1 400 Error\r\n";
                                stream.write_all(response.as_bytes()).await.unwrap();
                            }
                        }
                    }
                }
            });
        }
    }
}
