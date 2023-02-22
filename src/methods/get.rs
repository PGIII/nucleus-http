use crate::request;
use std::{
    net::{TcpStream}, 
    io::{prelude::*}, 
    fs
};

pub fn handle(r: &request::Request) -> String {
    //Check path and send correct file
    let file_name;
    if r.path() == "/" {
        //Send Index.html
        file_name = "index.html";
    } else {
        file_name = r.path();
    }

    let body;
    let status;

    //try to read file, 404 if not found
    if let Ok(contents) = fs::read_to_string(file_name) {
        body = contents;
        status = r.version().ok();
    } else {
        body = fs::read_to_string("404.html").unwrap();
        status = r.version().ok();
    }

    let length = body.len();
    let response = format!("{status}Content-Length: {length}\r\n\r\n{body}");
    return response;
}