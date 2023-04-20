use std::path::PathBuf;

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Method {
    GET,
    POST,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Version {
    V0_9,
    V1_0,
    V1_1,
    V2_0,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum StatusCode {
    Continue = 100,
    OK = 200,
    MovedPermanetly = 301,
    ErrBadRequest = 400,
    ErrUnathorized = 401,
    ErrForbidden = 403,
    ErrNotFound = 404,
    ErrInternalServer = 500,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MimeType {
    HTML,
    PlainText,
    JavaScript,
    Json,
    CSS,
    SVG,
    Icon,
    Binary,
    JPEG,
}

/// HTTP headers are simple key value pairs both strings
#[derive(Debug, PartialEq, Clone)]
pub struct Header {
    pub key: String,
    pub value: String,
}

impl From<&MimeType> for String {
    fn from(mime: &MimeType) -> String {
        let media_type = mime.media_type();
        let charset = mime.charset();
        let boundary = mime.boundary();
        if let (Some(boundary), Some(charset)) = (boundary, charset) {
            format!("{}; charset={}; boundary={}", media_type, charset, boundary)
        } else if let (None, Some(charset)) = (boundary, charset) {
            format!("{}; charset={}", media_type, charset)
        } else {
            format!("{};", media_type)
        }
    }
}

impl From<MimeType> for String {
    fn from(mime: MimeType) -> String {
        return String::from(&mime);
    }
}

impl From<PathBuf> for MimeType {
    fn from(value: PathBuf) -> Self {
        if let Some(ext) = value.extension() {
            return MimeType::from_extension(&ext.to_string_lossy());
        } else {
            return MimeType::PlainText;
        }
    }
}

impl MimeType {
    pub fn media_type(&self) -> &str {
        match self {
            Self::PlainText => "text/plain",
            Self::HTML => "text/html",
            Self::JavaScript => "text/javascript",
            Self::Json => "application/json",
            Self::CSS => "text/css",
            Self::SVG => "image/svg+xml",
            Self::Icon => "image/vnd.microsoft.icon",
            Self::Binary => "application/octet-stream",
            Self::JPEG => "image/jpeg",
        }
    }

    pub fn charset(&self) -> Option<&str> {
        match self {
            Self::SVG | Self::Icon | Self::Binary | Self::JPEG => None,
            _ => Some("utf-8"),
        }
    }

    pub fn boundary(&self) -> Option<&str> {
        match self {
            _ => None,
        }
    }

    pub fn from_extension(extension: &str) -> Self {
        match extension {
            "json" => Self::Json,
            "js" => Self::JavaScript,
            "css" => Self::CSS,
            "svg" => Self::SVG,
            "ico" => Self::Icon,
            "bin" => Self::Binary,
            "html" => Self::HTML,
            "jpeg" | "jpg" => Self::JPEG,
            _ => Self::PlainText,
        }
    }
}

impl TryFrom<String> for Header {
    type Error = &'static str;
    fn try_from(string: String) -> Result<Self, Self::Error> {
        let split: Vec<&str> = string.split(": ").collect();
        if split.len() == 2 {
            let key = split[0].to_string();
            let value = split[1].to_string();
            return Ok(Self { key, value });
        } else if split.len() > 2 {
            Err("Too many ': '")
        } else {
            Err("Invalid Key Value Pair")
        }
    }
}

impl TryFrom<&String> for Header {
    type Error = &'static str;
    fn try_from(string: &String) -> Result<Self, Self::Error> {
        let split: Vec<&str> = string.split(": ").collect();
        if split.len() == 2 {
            let key = split[0].to_string();
            let value = split[1].to_string();
            return Ok(Self { key, value });
        } else if split.len() > 2 {
            Err("Too many ': '")
        } else {
            Err("Invalid Key Value Pair")
        }
    }
}

impl From<&Header> for String {
    fn from(header: &Header) -> String {
        return format!("{}: {}", header.key, header.value);
    }
}

impl From<Header> for String {
    fn from(header: Header) -> String {
        return format!("{}: {}", header.key, header.value);
    }
}

impl Header {
    pub fn new(key: &str, value: &str) -> Header {
        Header {
            key: key.to_string(),
            value: value.to_string(),
        }
    }
    /// Create new vector of headers for server
    pub fn new_server() -> Vec<Header> {
        const VERSION: &str = env!("CARGO_PKG_VERSION");
        const NAME: &str = env!("CARGO_PKG_NAME");
        return vec![Header {
            key: "Server".to_string(),
            value: format!("{NAME} {VERSION}").to_string(),
        }];
    }
}

impl From<StatusCode> for &str {
    fn from(status: StatusCode) -> &'static str {
        match status {
            StatusCode::Continue => "100 Continue",
            StatusCode::OK => "200 OK",
            StatusCode::MovedPermanetly => "301 Moved Permantely",
            StatusCode::ErrUnathorized => "401 Unathorized",
            StatusCode::ErrForbidden => "403 Forbidden",
            StatusCode::ErrNotFound => "404 Not Found",
            _ => "500 Internal Server Error",
        }
    }
}

impl From<StatusCode> for String {
    fn from(status: StatusCode) -> String {
        let status_str: &str = status.into();
        return status_str.to_owned();
    }
}

impl Version {
    pub fn to_string(&self) -> String {
        match self {
            Version::V0_9 => "".to_owned(),
            Version::V1_0 => "HTTP/1.0".to_owned(),
            Version::V1_1 => "HTTP/1.1".to_owned(),
            Version::V2_0 => "HTTP/2".to_owned(),
        }
    }
}

impl From<Version> for &str {
    fn from(version: Version) -> &'static str {
        match version {
            Version::V0_9 => "",
            Version::V1_0 => "HTTP/1.0",
            Version::V1_1 => "HTTP/1.1",
            Version::V2_0 => "HTTP/2",
        }
    }
}

impl From<Version> for String {
    fn from(version: Version) -> String {
        return version.to_string();
    }
}
