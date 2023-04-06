use tokio::{
    self,
    io::{AsyncBufReadExt, AsyncWriteExt},
};

pub mod thread_pool;
pub mod request;
pub mod methods;

pub async fn serve(listener: tokio::net::TcpListener) -> tokio::io::Result<()> {
    loop {
        let (mut stream, _) = listener.accept().await?;

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
                Ok(r) => match r.method() {
                    request::Method::GET => {
                        let response = methods::get::handle(&r);
                        stream.write_all(response.as_bytes()).await.unwrap();
                    }
                    request::Method::POST => {}
                },
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
