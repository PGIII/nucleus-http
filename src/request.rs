#[derive(PartialEq, Debug)]
pub struct Request {
    method: Method,
    path: String,
    version: Version,
    host: String,
    headers: Option<Vec<String>>,
}

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
pub enum Error {
    InvalidString,
    InvalidMethod,
    InvalidHTTPVersion,
    MissingBlankLine,
    NoHostHeader,
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

    pub fn method(&self) -> Method {
        self.method
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

    pub fn get_header_value(&self, header_name: &str) -> Option<String> {
        return Self::header_value(&self.headers, header_name);
    }

    pub fn header_value(headers: &Option<Vec<String>>, header_name: &str) -> Option<String> {
        let mut value = None;
        if let Some(header_vec) = headers {
            for header in header_vec {
                let split: Vec<&str> = header.split(":").collect();
                if split.len() >= 2 {
                    let key = split[0];
                    if key == header_name {
                        value = Some(split[1].to_string());
                    }
                }
            }
        }
        return value;
    }

    pub fn from_lines(lines: &Vec<String>) -> Result<Request, Error> {
        let method;
        let version;
        let path;
        let mut headers = None;
        let host;

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
        path = request_seperated[1].to_string();

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
            let mut header_string = vec![];
            for i in 1..lines.len() {
                header_string.push(lines[i].to_string());
            }
            headers = Some(header_string);
        }

        let op_host = Self::header_value(&headers, "Host");
        if let Some(hostname) = op_host {
            host = hostname.trim().to_string();
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
        return Request::from_lines(&lines_string);
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
            headers: Some(vec!["Host: test".to_string()]),
            host: "test".to_string(),
        };
        let request = Request::from_string("GET / HTTP/1.1\r\nHost: test\r\n\r\n".to_owned())
            .expect("Error Parsing");
        assert_eq!(expected, request);
    }

    #[test]
    fn new_headers() {
        let expected = Request {
            method: Method::GET,
            version: Version::V1_1,
            path: "/".to_string(),
            headers: Some(vec![
                "Host: test".to_string(),
                "Header1: hi".to_string(),
                "Header2: Bye".to_string(),
            ]),
            host: "test".to_string(),
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
