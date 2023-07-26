use core::fmt;
use enum_map::Enum;
use memchr::memmem;
use std::path::PathBuf;

#[derive(PartialEq, Debug, Clone, Copy, Enum)]
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

pub type StatusCode = http::StatusCode;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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
    PNG,
    Custom(&'static str),
}

impl TryFrom<&[u8]> for Method {
    type Error = String;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        match value {
            b"GET" => Ok(Method::GET),
            b"POST" => Ok(Method::POST),
            _ => Err("Invalid Method".to_owned()),
        }
    }
}

impl TryFrom<&[u8]> for Version {
    type Error = String;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        match value {
            b"HTTP/1.0" => Ok(Version::V1_0),
            b"HTTP/1.1" => Ok(Version::V1_1),
            b"HTTP/2.2" => Ok(Version::V2_0),
            _ => Err("invalid version".to_owned()),
        }
    }
}

/// HTTP headers are simple key value pairs both strings
#[derive(Debug, PartialEq, Clone)]
pub struct Header {
    pub key: String,
    pub value: String,
}

pub trait IntoHeader {
    fn into_header(self) -> Header;
}

impl IntoHeader for Header {
    fn into_header(self) -> Header {
        self
    }
}

impl IntoHeader for &Header {
    fn into_header(self) -> Header {
        self.clone()
    }
}

impl IntoHeader for (&str, &str) {
    fn into_header(self) -> Header {
        let (key, value) = self;
        Header::new(key, value)
    }
}

impl IntoHeader for (&str, &String) {
    fn into_header(self) -> Header {
        let (key, value) = self;
        Header::new(key, value)
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GET => write!(f, "GET"),
            Self::POST => write!(f, "POST"),
        }
    }
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
        String::from(&mime)
    }
}

impl From<PathBuf> for MimeType {
    fn from(value: PathBuf) -> Self {
        if let Some(ext) = value.extension() {
            return MimeType::from_extension(&ext.to_string_lossy());
        } else {
            MimeType::PlainText
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
            Self::PNG => "image/png",
            Self::Custom(str) => str,
        }
    }

    pub fn charset(&self) -> Option<&str> {
        match self {
            Self::SVG | Self::Icon | Self::Binary | Self::JPEG => None,
            _ => Some("utf-8"),
        }
    }

    pub fn boundary(&self) -> Option<&str> {
        None
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
            "png" => Self::PNG,
            _ => Self::PlainText,
        }
    }
}

impl TryFrom<String> for Header {
    type Error = &'static str;
    fn try_from(string: String) -> Result<Self, Self::Error> {
        let split: Vec<&str> = string.split(": ").collect();
        match split.len().cmp(&2) {
            std::cmp::Ordering::Equal => {
                let key = split[0].to_lowercase();
                let value = split[1].to_string();
                Ok(Self { key, value })
            }
            std::cmp::Ordering::Greater => Err("Too many ': '"),
            std::cmp::Ordering::Less => Err("Invalid Key Value Pair"),
        }
    }
}

impl TryFrom<&String> for Header {
    type Error = &'static str;
    fn try_from(string: &String) -> Result<Self, Self::Error> {
        let split: Vec<&str> = string.split(": ").collect();
        match split.len().cmp(&2) {
            std::cmp::Ordering::Equal => {
                let key = split[0].to_lowercase();
                let value = split[1].to_string();
                Ok(Self { key, value })
            }
            std::cmp::Ordering::Greater => Err("Too many ': '"),
            std::cmp::Ordering::Less => Err("Invalid Key Value Pair"),
        }
    }
}

impl TryFrom<&str> for Header {
    type Error = &'static str;
    fn try_from(string: &str) -> Result<Self, Self::Error> {
        let split: Vec<&str> = string.split(": ").collect();
        match split.len().cmp(&2) {
            std::cmp::Ordering::Equal => {
                let key = split[0].to_lowercase();
                let value = split[1].to_string();
                Ok(Self { key, value })
            }
            std::cmp::Ordering::Greater => Err("Too many ': '"),
            std::cmp::Ordering::Less => Err("Invalid Key Value Pair"),
        }
    }
}

impl TryFrom<&[u8]> for Header {
    type Error = &'static str;
    fn try_from(h_str: &[u8]) -> Result<Self, Self::Error> {
        let sep = b": ";
        let key_end = memmem::find(h_str, sep).ok_or("missing ': '")?;
        let value_start = key_end + sep.len();
        let key = &h_str[0..key_end];
        let value = &h_str[value_start..h_str.len()];
        Ok(Self {
            key: String::from_utf8_lossy(key).to_string().to_lowercase(),
            value: String::from_utf8_lossy(value).to_string(),
        })
    }
}

impl From<&Header> for String {
    fn from(header: &Header) -> String {
        format!("{}: {}", header.key, header.value)
    }
}

impl From<Header> for String {
    fn from(header: Header) -> String {
        format!("{}: {}", header.key, header.value)
    }
}

impl Header {
    pub fn new(key: &str, value: &str) -> Header {
        Header {
            key: key.to_lowercase(),
            value: value.to_string(),
        }
    }
    /// Create new vector of headers for server
    pub fn new_server() -> Vec<Header> {
        const VERSION: &str = env!("CARGO_PKG_VERSION");
        const NAME: &str = env!("CARGO_PKG_NAME");
        vec![Header {
            key: "Server".to_string(),
            value: format!("{NAME} {VERSION}"),
        }]
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Version::V0_9 => "",
            Version::V1_0 => "HTTP/1.0",
            Version::V1_1 => "HTTP/1.1",
            Version::V2_0 => "HTTP/2",
        };
        write!(f, "{}", s)
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
        version.to_string()
    }
}
