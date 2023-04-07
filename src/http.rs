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

pub enum MimeType {
    PlainText,
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
