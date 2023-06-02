use crate::request;
use tokio::fs;

pub async fn handle(r: &request::Request) -> String {
    //Check path and send correct file

    let file_name = if r.path() == "/" {
        "index.html"
    } else {
        r.path()
    };

    let body;
    let status;

    //try to read file, 404 if not found
    if let Ok(contents) = fs::read_to_string(file_name).await {
        body = contents;
        status = r.ok();
    } else {
        body = fs::read_to_string("404.html").await.unwrap();
        status = r.ok();
    }

    let length = body.len();
    let response = format!("{status}Content-Length: {length}\r\n\r\n{body}");
    response
}

pub async fn load_file(r: &request::Request, file_path: &str) -> String {
    let body;
    let status;
    if let Ok(contents) = fs::read_to_string(file_path).await {
        body = contents;
        status = r.ok();
    } else {
        body = "file Not found".to_owned();
        status = r.error(404, "Not Found");
    }
    response(body, status)
}

pub fn response(body: String, status: String) -> String {
    let length = body.len();
    let response = format!("{status}Content-Length: {length}\r\n\r\n{body}");
    response
}
