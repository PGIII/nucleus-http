
use rust_web_server::{
    request,
    methods
};
use tokio::{self, io::{AsyncBufReadExt, AsyncWriteExt}};

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:7878").await?;

    loop {
        let (mut stream, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf_reader = tokio::io::BufReader::new(&mut stream);
            let mut lines = vec![];
            loop {
                let mut line: String = "".to_string();
                let result = buf_reader.read_line(&mut line).await;
                match result {
                    Ok(_) => {
                        if &line == "\r\n" {
                            break;
                        } else {
                            if line.ends_with("\r\n") {
                                line.pop();
                                line.pop();
                            }
                            lines.push(line);
                        }
                    },
                    Err(_) => {break;}
                }
            }
            let request_result = request::Request::from_lines(&lines);
            match request_result {
                Ok(r) => {
                    match r.method() {
                        request::Method::GET => {
                            let response = methods::get::handle(&r);
                            stream.write_all(response.as_bytes()).await.unwrap();
                        },
                        request::Method::POST => {},
                    }
                },
                Err(e) => {
                    dbg!(lines);
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


// fn _multithread_init() {
//     let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
//     let pool = ThreadPool::new(20);
    
//     println!("Running On localhost:7878");
//     for stream in listener.incoming() {
//         let stream = stream.unwrap();
        
//         pool.execute(|| {
//             _handle_connnection(stream);
//         });
//     }
//     println!("Shutting Down");
// }

// fn _handle_connnection(mut stream: TcpStream) {
//     let buf_reader = BufReader::new(&mut stream);
//     let http_request: Vec<_> = buf_reader
//         .lines()
//         .map(|result| result.unwrap())
//         .take_while(|line| !line.is_empty())
//         .collect();
//     let request_result = request::Request::from_lines(http_request);
//     match request_result {
//         Ok(r) => {
//             match r.method() {
//                 request::Method::GET => methods::get::handle(&r, &stream),
//                 request::Method::POST => {},
//             }
//         },
//         Err(e) => {
//             match e {
//                 _ => {
//                     let response = "HTTP/1.1 400 Error\r\n";
//                     stream.write_all(response.as_bytes()).unwrap();
//                 }  
//             }
//         }
//     }
        
    

    
//     // let (status_line, file_name) = match &request_line[..] {
//     //     "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "index.html"),
//     //     "GET /sleep HTTP/1.1" => {
//     //         thread::sleep(Duration::from_secs(5));
//     //         ("HTTP/1.1 200 OK", "index.html")
//     //     }
//     //     _ => ("HTTP/1.1 404 NOT FOUND", "404.html")
//     // };

//     // let contents = fs::read_to_string(file_name).unwrap();
//     // let length = contents.len();
//     // let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
//     // stream.write_all(response.as_bytes()).unwrap();

// }