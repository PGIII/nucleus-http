use std::path::PathBuf;
use tokio::fs;

use crate::http::{Header, MimeType, StatusCode, Version};

pub struct Response {
    version: Version,
    status: StatusCode,
    body: String, //for now only string, but could be other types eg. byte array
    mime: MimeType,
    headers: Vec<Header>,
}

impl Response {
    pub fn new(status: StatusCode, body: String, mime: MimeType) -> Response {
        let version = Version::V1_1;
        Response {
            status,
            mime,
            body,
            version,
            headers: Header::new_server(),
        }
    }

    pub fn error(status: StatusCode, body: String) -> Response {
        let version = Version::V1_1;
        let mime = MimeType::PlainText;
        Response {
            status,
            body,
            version,
            mime,
            headers: Header::new_server(),
        }
    }

    pub fn set_mime(&mut self, mime: MimeType) {
        self.mime = mime;
    }
}

impl From<String> for Response {
    fn from(string: String) -> Self {
        Response {
            status: StatusCode::OK,
            body: string,
            mime: MimeType::PlainText,
            version: Version::V1_1,
            headers: Header::new_server(),
        }
    }
}

impl From<&str> for Response {
    fn from(string: &str) -> Self {
        Response {
            status: StatusCode::OK,
            body: string.to_string(),
            mime: MimeType::PlainText,
            version: Version::V1_1,
            headers: Header::new_server(),
        }
    }
}

impl From<Response> for String {
    fn from(response: Response) -> String {
        let status: &str = response.status.into();
        let length = response.body.len();
        let version: &str = response.version.into();
        let body: &str = &response.body;
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
        return response;
    }
}
