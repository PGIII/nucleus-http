use core::fmt;
use std::collections::HashMap;

use crate::http::{Header, Method, Version};

#[derive(PartialEq, Debug, Clone)]
pub struct Request {
    method: Method,
    path: String,
    version: Version,
    host: String,
    query_string: Option<String>,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Error {
    InvalidString,
    InvalidMethod,
    InvalidHTTPVersion,
    MissingBlankLine,
    NoHostHeader,
    InvalidContentLength,
    WaitingOnBody,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", String::from(self))
    }
}
impl From<&Error> for String {
    fn from(value: &Error) -> Self {
        match value {
            Error::InvalidString => "Invalid String".to_string(),
            Error::NoHostHeader => "No VHost Specified".to_string(),
            Error::InvalidMethod => "Invalid Method Requested".to_string(),
            Error::InvalidHTTPVersion => "Unsupported HTTP version Request".to_string(),
            Error::MissingBlankLine => "Missing Blank Line".to_string(),
            Error::WaitingOnBody => "Waiting On Body".to_string(),
            Error::InvalidContentLength => "Content Length Invalid".to_string()
        }
    }
}

impl From<Error> for String {
    fn from(value: Error) -> Self {
        String::from(&value)
    }
}

impl Request {
    pub fn ok(&self) -> String {
        match self.version {
            Version::V0_9 => "200 OK\r\n".to_owned(),
            Version::V1_0 => "HTTP/1.0 200 OK\r\n".to_owned(),
            Version::V1_1 => "HTTP/1.1 200 OK\r\n".to_owned(),
            Version::V2_0 => "HTTP/2 200 OK\r\n".to_owned(),
        }
    }

    pub fn error(&self, code: u32, message: &str) -> String {
        return format!("{} {} {}\r\n", self.version.to_string(), code, message);
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn hostname(&self) -> &str {
        &self.host
    }

    pub fn body(&self) -> &Vec<u8> {
        &self.body
    }

    pub fn get_header_value(&self, header_name: &str) -> Option<String> {
        return Self::header_value(&self.headers, header_name);
    }

    pub fn header_value(headers: &HashMap<String, String>, header_name: &str) -> Option<String> {
        return headers.get(header_name).cloned();
    }

    pub fn from_lines(lines: &Vec<String>) -> Result<Request, Error> {
        let method;
        let version;
        let path;
        let mut headers = HashMap::new();
        let host;
        let mut query_string = None;
        let body = vec![];

        let request_seperated: Vec<&str> = lines[0].split(" ").collect(); //First line is request
        if request_seperated.len() < 3 {
            return Err(Error::InvalidString);
        }

        //First is method
        match request_seperated[0] {
            "GET" => method = Method::GET,
            "POST" => method = Method::POST,
            _ => return Err(Error::InvalidMethod),
        }

        //second string is url
        let url = request_seperated[1].to_string();
        let url_split: Vec<&str> = url.split('?').collect(); //anything after ? is query string
        path = url_split[0].to_string();
        if url_split.len() > 1 {
            query_string = Some(url_split[1].to_string());
        }

        //third is http Verison
        match request_seperated[2] {
            "HTTP/1.0" => version = Version::V1_0,
            "HTTP/1.1" => version = Version::V1_1,
            "HTTP/2.2" => version = Version::V2_0,
            _ => return Err(Error::InvalidHTTPVersion),
        }

        //4th is optional headers
        if lines.len() > 1 {
            //FIXME: Dont we need to collect here?
            for i in 1..lines.len() {
                if let Ok(header) = Header::try_from(&lines[i]) {
                    headers.insert(header.key, header.value);
                }
                //headers.push(lines[i].to_string());
            }
        }

        let op_host = Self::header_value(&headers, "Host");
        if let Some(hostname) = op_host {
            // get rid if port if its included in host name
            let hostname_only: Vec<&str> = hostname.split(":").collect();
            host = hostname_only[0].to_string();
        } else {
            //FIXME: should we only error when its > http 1.0????
            return Err(Error::NoHostHeader);
        }
        //last is optional headers
        return Ok(Request {
            method,
            version,
            path,
            headers,
            host,
            query_string,
            body,
        });
    }

    pub fn from_string(request_str: String) -> Result<Request, Error> {
        //Make sure its not an empty string and has at least one line
        if request_str.len() == 0 {
            return Err(Error::InvalidString);
        }

        let blank_line_split: Vec<&str> = request_str.split("\r\n\r\n").collect();
        let lines: Vec<&str> = blank_line_split[0].split("\r\n").collect();

        if blank_line_split.len() == 1 {
            return Err(Error::MissingBlankLine);
        }
        let lines_string: Vec<String> = lines.iter().map(|&s| s.to_string()).collect();
        let request = Request::from_lines(&lines_string);
        if let Ok(mut req) = request.clone() {
            //check for content length header
            if let Some(content_lenth) = req.get_header_value("Content-Length") {
                if let Ok(len) = content_lenth.parse() {
                    if blank_line_split[1].len() < len {
                        return Err(Error::WaitingOnBody);
                    } else {
                        req.body.extend_from_slice(blank_line_split[1].as_bytes());
                        return Ok(req);
                    }
                } else {
                    return Err(Error::InvalidContentLength);
                }
            }
        }

        return request;
    }

    pub fn query_string(&self) -> Option<&String> {
        self.query_string.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_wrong_version_new() {
        let expected = Err(Error::InvalidHTTPVersion);
        let request = Request::from_string("GET / HTTP1.1\r\n\r\n".to_owned());
        assert_eq!(expected, request);
    }

    #[test]
    fn no_blank_line_new() {
        let expected = Err(Error::MissingBlankLine);
        let request = Request::from_string("GET / HTTP/1.1".to_owned());
        assert_eq!(expected, request);
    }

    #[test]
    fn new() {
        let expected = Request {
            method: Method::GET,
            version: Version::V1_1,
            path: "/".to_string(),
            body: vec![],
            headers: HashMap::from([("Host".to_string(), "test".to_string())]),
            host: "test".to_string(),
            query_string: None,
        };
        let request = Request::from_string("GET / HTTP/1.1\r\nHost: test\r\n\r\n".to_owned())
            .expect("Error Parsing");
        assert_eq!(expected, request);
    }

    #[test]
    fn new_query_string() {
        let expected = Request {
            method: Method::GET,
            version: Version::V1_1,
            path: "/index.html".to_string(),
            body: vec![],
            headers: HashMap::from([("Host".to_string(), "test".to_string())]),
            host: "test".to_string(),
            query_string: Some("test=true".to_string()),
        };
        let request = Request::from_string(
            "GET /index.html?test=true HTTP/1.1\r\nHost: test\r\n\r\n".to_owned(),
        )
        .expect("Error Parsing");
        assert_eq!(expected, request);
    }

    #[test]
    fn new_headers() {
        let expected = Request {
            method: Method::GET,
            version: Version::V1_1,
            path: "/".to_string(),
            body: vec![],
            headers: HashMap::from([
                ("Host".to_string(), "test".to_string()),
                ("Header1".to_string(), "hi".to_string()),
                ("Header2".to_string(), "Bye".to_string()),
            ]),
            host: "test".to_string(),
            query_string: None,
        };
        let request = Request::from_string(
            "GET / HTTP/1.1\r\nHost: test\r\nHeader1: hi\r\nHeader2: Bye\r\n\r\n".to_owned(),
        )
        .expect("Error Parsing");
        assert_eq!(expected, request);
    }

    #[test]
    fn empty_string() {
        let request = Request::from_string("".to_owned());
        if let Err(error) = request {
            if error == Error::InvalidString {
                assert!(true);
            } else {
                assert!(false);
            }
        } else {
            assert!(false);
        }
    }
}
