use crate::{
    http::{Header, Method, Version},
    utils,
};
use bytes::Bytes;
use core::fmt;
use memchr::{memchr, memchr_iter, memmem};
use std::{collections::HashMap, format, vec};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormTypes {
    None,
    MultiPart(HashMap<String, MultiPartFormEntry>),
    XUrlEncoded(HashMap<String, String>), //simple string key value pair
}

#[derive(PartialEq, Debug, Clone)]
pub struct Request {
    method: Method,
    path: String,
    version: Version,
    host: String,
    query_string: Option<String>,
    headers: HashMap<String, String>,
    body: Vec<u8>,
    form_data: FormTypes,
    keep_alive: bool,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Error {
    InvalidString,
    InvalidMethod,
    InvalidHTTPVersion,
    MissingBlankLine,
    NoHostHeader,
    InvalidContentLength,
    WaitingOnBody(Option<usize>), // this can return number of bytes left in body
    MissingMultiPartBoundary,
    MissingContentLength,
    InvalidUrlEncodedForm,
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
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
            Error::WaitingOnBody(_) => "Waiting On Body".to_string(),
            Error::InvalidContentLength => "Content Length Invalid".to_string(),
            Error::MissingMultiPartBoundary => "Missing Mulipart boundary".to_string(),
            Error::MissingContentLength => "Missing Content Length Header".to_string(),
            Error::InvalidUrlEncodedForm => "Invalid URL Encoded Form".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiPartFormEntry {
    name: String,
    file_name: Option<String>,
    content_type: Option<String>,
    value: Vec<u8>,
}

impl MultiPartFormEntry {
    pub fn from_string(form_str: &str) -> Result<MultiPartFormEntry, anyhow::Error> {
        //first split out body
        let split: Vec<_> = form_str.split("\r\n\r\n").collect();
        if let (Some(header), Some(body)) = (split.first(), split.get(1)) {
            let lines = header.split("\r\n");
            let mut form_args: HashMap<&str, &str> = HashMap::new();
            let mut content_type = None;
            for line in lines {
                let name_value_split: Vec<_> = line.split(": ").collect();
                if let (Some(header_name), Some(header_value)) =
                    (name_value_split.first(), name_value_split.get(1))
                {
                    match header_name.to_lowercase().as_str() {
                        "content-type" => {
                            content_type = Some(header_value.to_string());
                        }
                        "content-disposition" => {
                            let split = header_value.split("; ");
                            for op in split {
                                let nv: Vec<_> = op.split('=').collect();
                                if let (Some(n), Some(v)) = (nv.first(), nv.get(1)) {
                                    form_args.insert(n, strip_quotes(v));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            if let Some(name) = form_args.get("name") {
                let name = name.to_string();
                let file = form_args.get("filename").map(|s| s.to_string());
                Ok(MultiPartFormEntry {
                    name,
                    file_name: file,
                    content_type,
                    value: body.as_bytes().into(),
                })
            } else {
                Err(anyhow::Error::msg("Missing Name"))
            }
        } else {
            Err(anyhow::Error::msg("Missing Body"))
        }
    }

    pub fn from_bytes(form: &[u8]) -> Result<MultiPartFormEntry, anyhow::Error> {
        //first split out body
        if let Some(blank_line) = memmem::find(form, b"\r\n\r\n") {
            let mut form_args: HashMap<String, String> = HashMap::new();
            let mut content_type = None;
            let header = &form[0..blank_line + 2];
            let mut body = &form[blank_line + 4..];
            if body[body.len() - 1] == b'\n' && body[body.len() - 2] == b'\r' {
                body = &body[0..body.len() - 2]; // trim crlf at end of body if there
            }
            let newline_iter = memmem::find_iter(header, "\r\n");
            let mut last_header_start = 0;
            for i in newline_iter {
                let new_header = &header[last_header_start..i];
                last_header_start = i + 2;
                if let Some(colon_i) = memmem::find(new_header, b": ") {
                    let name_b = &new_header[0..colon_i];
                    let name = String::from_utf8_lossy(name_b);
                    let value = &new_header[colon_i + 2..];
                    match name.to_lowercase().as_str() {
                        "content-type" => {
                            content_type = Some(String::from_utf8_lossy(value).to_string());
                        }
                        "content-disposition" => {
                            let header_value = String::from_utf8_lossy(value).to_string();
                            let split = header_value.split("; ");
                            for op in split {
                                let nv: Vec<_> = op.split('=').collect();
                                if let (Some(n), Some(v)) = (nv.first(), nv.get(1)) {
                                    form_args.insert(n.to_string(), strip_quotes(v).to_string());
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            if let Some(name) = form_args.get("name") {
                let name = name.to_string();
                let file = form_args.get("filename").map(|s| s.to_string());
                Ok(MultiPartFormEntry {
                    name,
                    file_name: file,
                    content_type,
                    value: body.into(),
                })
            } else {
                Err(anyhow::Error::msg("Missing Name"))
            }
        } else {
            Err(anyhow::Error::msg("Missing Body"))
        }
    }

    pub fn name_value(name: &str, value: &str) -> Self {
        MultiPartFormEntry {
            name: name.to_string(),
            file_name: None,
            content_type: None,
            value: value.to_string().into(),
        }
    }

    pub fn file(name: &str, file_name: &str, value: &str) -> Self {
        MultiPartFormEntry {
            name: name.to_string(),
            file_name: Some(file_name.to_string()),
            content_type: None,
            value: value.to_string().into(),
        }
    }

    pub fn file_name(&self) -> Option<&String> {
        self.file_name.as_ref()
    }

    pub fn content_type(&self) -> Option<&String> {
        self.content_type.as_ref()
    }

    pub fn value(&self) -> &[u8] {
        self.value.as_ref()
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", String::from(self))
    }
}

impl From<Error> for String {
    fn from(value: Error) -> Self {
        String::from(&value)
    }
}

fn get_boundary(content_type_value_str: &str) -> Result<&str, anyhow::Error> {
    let parts: Vec<_> = content_type_value_str.split(';').collect();
    if parts.len() > 1 {
        let nv: Vec<_> = parts[1].split('=').collect();
        if let Some(boundary) = nv.get(1) {
            Ok(strip_quotes(boundary))
        } else {
            Err(anyhow::Error::msg("Invalid boundary"))
        }
    } else {
        Err(anyhow::Error::msg("Boundary Missing from string"))
    }
}

fn get_multiparts_entries_from_bytes(
    body: &[u8],
    boundary: &[u8],
) -> anyhow::Result<HashMap<String, MultiPartFormEntry>> {
    let mut end_marker = vec![b'-', b'-'];
    end_marker.extend_from_slice(boundary);
    let boundary_marker = end_marker.clone();
    end_marker.extend_from_slice(b"--");
    if memmem::find(body, &end_marker).is_some() {
        //we have an en marker go through the bodies, body is anything after marker before next
        //marker or end boundary
        let mut body_spliter = memmem::find_iter(body, &boundary_marker);
        let mut entries = HashMap::new();
        if let Some(mut last_bound) = body_spliter.next() {
            last_bound += boundary_marker.len();
            for bound in body_spliter {
                let current_body = &body[last_bound..bound];
                last_bound = bound + boundary_marker.len();
                //FIXME: make this use bytes
                if let Ok(entry) = MultiPartFormEntry::from_bytes(current_body) {
                    entries.insert(entry.name.clone(), entry);
                }
            }
        } else {
            return Err(anyhow::Error::msg("Missing boundaries"));
        }
        Ok(entries)
    } else {
        Err(anyhow::Error::msg("Not Full Body"))
    }
}

fn strip_quotes(value: &str) -> &str {
    let split: Vec<_> = value.split('\"').collect();
    if let Some(v) = split.get(1) {
        v
    } else {
        split[0]
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
        format!("{} {} {}\r\n", self.version, code, message)
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
        let lower = header_name.to_lowercase();
        Self::header_value(&self.headers, &lower)
    }

    pub fn header_value(headers: &HashMap<String, String>, header_name: &str) -> Option<String> {
        let lower = header_name.to_lowercase();
        return headers.get(&lower).cloned();
    }

    pub fn keep_alive(&self) -> bool {
        self.keep_alive
    }

    pub fn from_lines(lines: &Vec<&str>) -> Result<Request, Error> {
        let mut headers = HashMap::new();
        let host;
        let mut query_string = None;
        let body = vec![];
        let form_data = FormTypes::None;

        let request_seperated: Vec<&str> = lines[0].split(' ').collect(); //First line is request
        if request_seperated.len() < 3 {
            return Err(Error::InvalidString);
        }

        //First is method
        let method = match request_seperated[0] {
            "GET" => Method::GET,
            "POST" => Method::POST,
            _ => return Err(Error::InvalidMethod),
        };

        //second string is url
        let url = request_seperated[1].to_string();
        let url_split: Vec<&str> = url.split('?').collect(); //anything after ? is query string
        let path = url_split[0].to_string();
        if url_split.len() > 1 {
            query_string = Some(url_split[1].to_string());
        }

        //third is http Verison
        let version = match request_seperated[2] {
            "HTTP/1.0" => Version::V1_0,
            "HTTP/1.1" => Version::V1_1,
            "HTTP/2.2" => Version::V2_0,
            _ => return Err(Error::InvalidHTTPVersion),
        };

        //4th is optional headers
        if lines.len() > 1 {
            //FIXME: Dont we need to collect here?
            for line in lines.iter().skip(1) {
                if let Ok(header) = Header::try_from(*line) {
                    headers.insert(header.key, header.value);
                }
                //headers.push(lines[i].to_string());
            }
        }

        let op_host = Self::header_value(&headers, "Host");
        if let Some(hostname) = op_host {
            // get rid if port if its included in host name
            let hostname_only: Vec<&str> = hostname.split(':').collect();
            host = hostname_only[0].to_string();
        } else {
            //FIXME: should we only error when its > http 1.0????
            return Err(Error::NoHostHeader);
        }
        //last is optional headers
        let keep_alive = Self::determine_keep_alive(version, headers.get("connection"));
        Ok(Request {
            method,
            version,
            path,
            headers,
            host,
            query_string,
            body,
            form_data,
            keep_alive,
        })
    }

    pub fn from_bytes(request_bytes: Bytes) -> Result<Request, Error> {
        let bytes = request_bytes;
        if bytes.is_empty() {
            return Err(Error::InvalidString);
        }
        //first split the header from the body, first \r\n\r\n should seperate that
        if let Some(blank_line_index) = memmem::find(&bytes, b"\r\n\r\n") {
            let req_header = bytes.slice(0..blank_line_index + 2); //include last crlf for easier
                                                                   //header parsing
            let mut req_body = bytes.slice(blank_line_index + 4..bytes.len());
            let mut req_header_lines = memmem::find_iter(&req_header, "\r\n");
            if let Some(i) = req_header_lines.next() {
                let url;
                let mut headers = HashMap::new(); 
                let host;
                let mut query_string = None;
                let mut form_data = FormTypes::None;
                let request_line = req_header.slice(0..i);
                let mut header_start = i + 2;
                let mut space_iter = memchr_iter(b' ', &request_line);
                let method_end = space_iter.next().ok_or(Error::InvalidString)?;
                let url_end = space_iter.next().ok_or(Error::InvalidString)?;
                let method_b = request_line.slice(0..method_end);
                let url_b = request_line.slice(method_end + 1..url_end);
                let version_b = request_line.slice(url_end + 1..request_line.len());

                let method: Method =
                    Method::try_from(method_b.as_ref()).map_err(|_| Error::InvalidMethod)?;
                let version =
                    Version::try_from(version_b.as_ref()).map_err(|_| Error::InvalidHTTPVersion)?;
                //check for query_string in url
                if let Some(qmark) = memchr(b'?', &url_b) {
                    let query = url_b.slice(qmark + 1..url_b.len());
                    let url_slice = url_b.slice(0..qmark);
                    query_string = Some(String::from_utf8_lossy(query.as_ref()).to_string());
                    url = String::from_utf8_lossy(url_slice.as_ref()).to_string();
                } else {
                    url = String::from_utf8_lossy(url_b.as_ref()).to_string();
                }

                // go through rest of the lines in header and parse out any headers
                for line_end in req_header_lines {
                    let header_line = req_header.slice(header_start..line_end);
                    header_start = line_end + 2;
                    if let Ok(header) = Header::try_from(header_line.as_ref()) {
                        headers.insert(header.key, header.value);
                    }
                }

                //make sure we have a host header
                let op_host = Self::header_value(&headers, "Host");
                if let Some(hostname) = op_host {
                    // get rid if port if its included in host name
                    let hostname_only: Vec<&str> = hostname.split(':').collect();
                    host = hostname_only[0].to_string();
                } else {
                    //FIXME: should we only error when its > http 1.0????
                    return Err(Error::NoHostHeader);
                }

                //lastly check we got the full body of the request
                if let Some(content_length) = Self::header_value(&headers, "Content-Length") {
                    if let Ok(len) = content_length.parse() {
                        if req_body.len() < len {
                            return Err(Error::WaitingOnBody(Some(len - req_body.len())));
                        }
                    } else {
                        return Err(Error::InvalidContentLength);
                    }
                }
                if let Some(content_type) = Self::header_value(&headers, "Content-Type") {
                    match content_type {
                        x if x.contains("multipart/form-data;") => {
                            match get_boundary(&x) {
                                Ok(boundary) => {
                                    match get_multiparts_entries_from_bytes(
                                        &req_body,
                                        boundary.as_bytes(),
                                    ) {
                                        Ok(entries) => {
                                            form_data = FormTypes::MultiPart(entries);
                                            req_body.clear(); //clear since we parsed it
                                                              //return Ok(req);
                                        }
                                        Err(_) => {
                                            return Err(Error::WaitingOnBody(None));
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::debug!("Error Parsing Boundary: {}", e.to_string());
                                    return Err(Error::MissingMultiPartBoundary);
                                }
                            }
                        }
                        x if x.contains("application/x-www-form-urlencoded") => {
                            //Parse here
                            match utils::parse_query_string(&req_body) {
                                Ok(form) => {
                                    form_data = FormTypes::XUrlEncoded(form);
                                    req_body.clear();
                                }
                                Err(e) => {
                                    log::error!(
                                        "Error Parsing URL Encoded Body: {}",
                                        e.to_string()
                                    );
                                    return Err(Error::InvalidUrlEncodedForm);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                let keep_alive = Self::determine_keep_alive(version, headers.get("connection"));
                Ok(Request {
                    method,
                    version,
                    path: url,
                    headers,
                    host,
                    query_string,
                    body: req_body.to_vec(),
                    form_data,
                    keep_alive,
                })
            } else {
                //no headers, we need at least the host header
                panic!("request parsing: Somehow missing CRLF even though CRLFCRLF was present");
            }
        } else {
            Err(Error::MissingBlankLine)
        }
    }

    pub fn from_string(request_str: String) -> Result<Request, Error> {
        let bytes = Bytes::from(request_str);
        Self::from_bytes(bytes)
    }

    pub fn query_string(&self) -> Option<&String> {
        self.query_string.as_ref()
    }

    pub fn form_data(&self) -> &FormTypes {
        &self.form_data
    }
    
    /// determin if request wants to keep connection alive
    /// if connection header present this value is controlled by that
    /// otherwise determined by default behavior for version passed
    fn determine_keep_alive(version: Version, connection_header: Option<&String>) -> bool {
        if let Some(conn) = connection_header {
            conn.to_lowercase() == "keep-alive"
        } else {
            // no conncection header so use version default 
            match version {
                Version::V0_9 => false,
                Version::V1_0 => false,
                Version::V1_1 => true,
                Version::V2_0 => true,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::assert_eq;

    use super::*;

    #[test]
    fn x_url_encoded_form() {
        let mut map = HashMap::new();
        map.insert("field1".to_string(), "value1".to_string());
        map.insert("field2".to_string(), "value2".to_string());

        let expected = Request {
            method: Method::POST,
            version: Version::V1_1,
            path: "/test".to_string(),
            body: vec![],
            headers: HashMap::from([
                ("host".to_string(), "foo.example".to_string()),
                (
                    "content-type".to_string(),
                    "application/x-www-form-urlencoded".to_string(),
                ),
                ("content-length".to_string(), "27".to_string()),
            ]),
            host: "foo.example".to_string(),
            query_string: None,
            form_data: FormTypes::XUrlEncoded(map),
            keep_alive: true,
        };
        let request_str = Bytes::from_static(
            b"POST /test HTTP/1.1\r\n\
            Host: foo.example\r\n\
            Content-Type: application/x-www-form-urlencoded\r\n\
            Content-Length: 27\r\n\r\n\
            field1=value1&field2=value2",
        ); //does this normally have CRLF here ?
        let request = Request::from_bytes(request_str).expect("Could not build request");
        assert_eq!(expected, request);
    }

    #[test]
    fn multipart_form() {
        let field1 = MultiPartFormEntry::name_value("field1", "value1");
        let field2 = MultiPartFormEntry::file("field2", "example.txt", "value2");
        let mut map = HashMap::new();
        map.insert(field1.name.clone(), field1);
        map.insert(field2.name.clone(), field2);
        let expected = Request {
            method: Method::POST,
            version: Version::V1_1,
            path: "/test".to_string(),
            body: vec![],
            headers: HashMap::from([
                ("host".to_string(), "foo.example".to_string()),
                (
                    "content-type".to_string(),
                    "multipart/form-data;boundary=\"boundary\"".to_string(),
                ),
            ]),
            host: "foo.example".to_string(),
            query_string: None,
            form_data: FormTypes::MultiPart(map),
            keep_alive: true,
        };
        let request_str = Bytes::from_static(
            b"POST /test HTTP/1.1\r\n\
        Host: foo.example\r\n\
        Content-Type: multipart/form-data;boundary=\"boundary\"\r\n\
        \r\n\
        --boundary\r\n\
        Content-Disposition: form-data; name=\"field1\"\r\n\
        \r\n\
        value1\r\n\
        --boundary\r\n\
        Content-Disposition: form-data; name=\"field2\"; filename=\"example.txt\"\r\n\
        \r\n\
        value2\r\n\
        --boundary--",
        );

        let request = Request::from_bytes(request_str).expect("Could not build request");
        assert_eq!(expected, request);
    }

    #[test]
    fn get_wrong_version_new() {
        let expected = Err(Error::InvalidHTTPVersion);
        let request =
            Request::from_bytes(Bytes::from_static(b"GET / HTTP1.1\r\nHost: test\r\n\r\n"));
        assert_eq!(expected, request);
    }

    #[test]
    fn no_blank_line_new() {
        let expected = Err(Error::MissingBlankLine);
        let req_str = Bytes::from_static(b"GET / HTTP/1.1");
        let request = Request::from_bytes(req_str);
        assert_eq!(expected, request);
    }

    #[test]
    fn new() {
        let expected = Request {
            method: Method::GET,
            version: Version::V1_1,
            path: "/".to_string(),
            body: vec![],
            headers: HashMap::from([("host".to_string(), "test".to_string())]),
            host: "test".to_string(),
            query_string: None,
            form_data: FormTypes::None,
            keep_alive: true,
        };
        let request =
            Request::from_bytes(Bytes::from_static(b"GET / HTTP/1.1\r\nHost: test\r\n\r\n"))
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
            headers: HashMap::from([("host".to_string(), "test".to_string())]),
            host: "test".to_string(),
            query_string: Some("test=true".to_string()),
            form_data: FormTypes::None,
            keep_alive: true,
        };
        let request = Request::from_bytes(Bytes::from_static(
            b"GET /index.html?test=true HTTP/1.1\r\nHost: test\r\n\r\n",
        ))
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
                ("host".to_string(), "test".to_string()),
                ("header1".to_string(), "hi".to_string()),
                ("header2".to_string(), "Bye".to_string()),
            ]),
            host: "test".to_string(),
            query_string: None,
            form_data: FormTypes::None,
            keep_alive: true,
        };
        let request = Request::from_string(
            "GET / HTTP/1.1\r\nhost: test\r\nheader1: hi\r\nheader2: Bye\r\n\r\n".to_owned(),
        )
        .expect("Error Parsing");
        assert_eq!(expected, request);
    }

    #[test]
    fn empty_string() {
        let request = Request::from_string("".to_owned());
        if let Err(error) = request {
            assert!(error == Error::InvalidString);
        } else {
            panic!("No error");
        }
    }
}
