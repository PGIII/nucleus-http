use std::{
    net::{TcpListener, TcpStream}, 
    io::{prelude::*, BufReader}, 
};
use rust_web_server::{
    thread_pool::ThreadPool,
    request,
    methods
};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let pool = ThreadPool::new(10);
    
    println!("Running On localhost:7878");
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        
        pool.execute(|| {
            handle_connnection(stream);
        });
    }
    println!("Shutting Down");
}

fn handle_connnection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);
    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();
    let request_result = request::Request::from_lines(http_request);
    match request_result {
        Ok(r) => {
            match r.method() {
                request::Method::GET => methods::get::handle(&r, &stream),
                request::Method::POST => {},
            }
        },
        Err(e) => {
            match e {
                _ => {
                    let response = "HTTP/1.1 400 Error\r\n";
                    stream.write_all(response.as_bytes()).unwrap();
                }  
            }
        }
    }
        
    

    
    // let (status_line, file_name) = match &request_line[..] {
    //     "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "index.html"),
    //     "GET /sleep HTTP/1.1" => {
    //         thread::sleep(Duration::from_secs(5));
    //         ("HTTP/1.1 200 OK", "index.html")
    //     }
    //     _ => ("HTTP/1.1 404 NOT FOUND", "404.html")
    // };

    // let contents = fs::read_to_string(file_name).unwrap();
    // let length = contents.len();
    // let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
    // stream.write_all(response.as_bytes()).unwrap();

}