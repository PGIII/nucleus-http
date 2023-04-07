use crate::http::{MimeType, StatusCode, Version};

pub struct Response {
    version: Version,
    status: StatusCode,
    body: String, //for now only string, but could be other types eg. byte array
    mime: MimeType,
}

impl Response {
    pub fn new(status: StatusCode, body: String, mime: MimeType, version: Version) -> Response {
        let version = Version::V1_1;
        Response {
            status,
            mime,
            body,
            version,
        }
    }
}

impl From<String> for Response {
    fn from(string: String) -> Self {
        Response {
            status: StatusCode::OK,
            body: string,
            mime: MimeType::PlainText,
            version: Version::V1_1,
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
        }
    }
}

impl From<Response> for String {
    fn from(response: Response) -> String {
        let status: &str = response.status.into();
        let length = response.body.len();
        let version: &str = response.version.into();
        let body: &str = &response.body;
        let response = format!("{version} {status} Content-Length: {length}\r\n\r\n{body}");
        return response;
    }
}
