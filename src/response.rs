use std::convert::Infallible;

use crate::http::{Header, MimeType, StatusCode, Version, IntoHeader};
use anyhow;

pub type ResponseBody = Vec<u8>;

#[derive(Debug, PartialEq)]
pub struct Response {
    version: Version,
    status: StatusCode,
    body: ResponseBody,
    mime: MimeType,
    headers: Vec<Header>,
}

pub trait IntoResponse {
    fn into_response(self) -> Response;
}

/// All types that implent into response already get IntoResponse for free ... does that make this
/// trait redudant ?
impl<T> IntoResponse for T
where
    T: Into<Response>,
{
    fn into_response(self) -> Response {
        self.into()
    }
}

impl Response {
    pub fn new(status: StatusCode, body: ResponseBody, mime: MimeType) -> Response {
        let version = Version::V1_1;
        Response {
            status,
            mime,
            body,
            version,
            headers: vec![],
        }
    }

    pub fn error(status: StatusCode, body: ResponseBody) -> Response {
        let version = Version::V1_1;
        let mime = MimeType::PlainText;
        Response {
            status,
            body,
            version,
            mime,
            headers: vec![],
        }
    }

    pub fn set_mime(&mut self, mime: MimeType) {
        self.mime = mime;
    }

    pub fn to_send_buffer(&self) -> Vec<u8> {
        //transform response to array of bytes to be sent
        let status: &str = &self.status.to_string();
        let length = self.body.len();
        let version: &str = self.version.into();
        let content_type: String = String::from(&self.mime);
        let mut headers_string = "".to_string();
        for header in &self.headers {
            let header_string: String = String::from(header);
            headers_string.push_str(&header_string);
            headers_string.push_str("\r\n");
        }
        let mut buffer: Vec<u8> = Vec::new();
        let response = format!(
            "{version} {status}\r\n\
            Content-Length: {length}\r\n\
            Content-Type: {content_type}\r\n\
            {headers_string}\r\n"
        );
        buffer.append(&mut response.into_bytes());
        for byte in &self.body {
            buffer.push(*byte);
        }
        buffer
    }

    pub fn add_header(&mut self, value: impl IntoHeader) {
        let header = value.into_header();
        self.headers.push(header);
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn mime(&self) -> MimeType {
        self.mime
    }
}

impl From<Vec<u8>> for Response {
    fn from(bytes: Vec<u8>) -> Self {
        Response {
            status: StatusCode::OK,
            body: bytes,
            mime: MimeType::Binary,
            version: Version::V1_1,
            headers: vec![],
        }
    }
}

impl From<String> for Response {
    fn from(string: String) -> Self {
        Response {
            status: StatusCode::OK,
            body: string.into(),
            mime: MimeType::HTML,
            version: Version::V1_1,
            headers: vec![],
        }
    }
}

impl From<&str> for Response {
    fn from(string: &str) -> Self {
        Response {
            status: StatusCode::OK,
            body: string.into(),
            mime: MimeType::HTML,
            version: Version::V1_1,
            headers: vec![],
        }
    }
}

impl From<anyhow::Error> for Response {
    fn from(value: anyhow::Error) -> Self {
        tracing::error!(?value);
       let message = "<h1>Error 500 Internal Server Error</h1>\r\n".to_string(); 
        Response {
            status: StatusCode::INTERNAL_SERVER_ERROR, //FIXME: make this smarter
            body: message.into(),
            mime: MimeType::HTML,
            version: Version::V1_1,
            headers: vec![],
        }
    }
}


impl From<Infallible> for Response {
    fn from(_: Infallible) -> Self {
        panic!("tried to conver from infalliable");
    }
}

impl From<Response> for String {
    fn from(response: Response) -> String {
        let status: &str = response.status.as_str();
        let length = response.body.len();
        let version: &str = response.version.into();
        let body: &str = &String::from_utf8_lossy(&response.body);
        let content_type: String = response.mime.into();
        let mut headers_string = "".to_string();
        for header in response.headers {
            let header_string: String = header.into();
            headers_string.push_str(&header_string);
            headers_string.push_str("\r\n");
        }
        let response = format!(
            "{version} {status}\r\n\
            Content-Length: {length}\r\n\
            Content-Type: {content_type}\r\n\
            {headers_string}\r\n\
            {body}"
        );
        response
    }
}

impl From<std::io::Error> for Response {
    fn from(error: std::io::Error) -> Self {
        Response {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            body: error.to_string().into(),
            mime: MimeType::HTML,
            version: Version::V1_1,
            headers: vec![],
        }
    }
}
