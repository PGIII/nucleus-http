#[derive(PartialEq, Debug)]
pub struct Request {
    method: Method,
    path: String,
    version: Version,
    headers: Option<Vec<String>>,
}

#[derive(PartialEq, Debug)]
enum Method {
    GET,
    POST,
}

#[derive(PartialEq, Debug)]
enum Version {
    V1_1,
    V2_0,
}

#[derive(PartialEq, Debug)]
pub enum Error {
    InvalidString,
    InvalidMethod,
    InvalidHTTPVersion,
    MissingBlankLine,
}

impl Request {
    pub fn from_lines(lines: Vec<&str>) -> Result<Request, Error> {
        let method;
        let version;
        let path;
        let mut headers = None;

        let request_seperated: Vec<&str> = lines[0].split(" ").collect();//First line is request
        if  request_seperated.len() < 3 {
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

        //last is optional headers
        return Ok(Request { 
            method,
            version,
            path,
            headers,
        })      
    }

    pub fn from_string(request_str: &str) -> Result<Request, Error> {
        //Make sure its not an empty string and has at least one line
        if request_str.len() == 0 {
            return Err(Error::InvalidString);
        }

        let blank_line_split: Vec<&str> = request_str.split("\r\n\r\n").collect();
        let lines: Vec<&str> = blank_line_split[0].split("\r\n").collect();

        if blank_line_split.len() == 1 {
            return Err(Error::MissingBlankLine);
        }
        return Request::from_lines(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_wrong_version_new() {
        let expected = Err(Error::InvalidHTTPVersion);
        let request = Request::from_string("GET / HTTP1.1\r\n\r\n");
        assert_eq!(expected, request);
    }
    
    #[test]
    fn no_blank_line_new() {
        let expected = Err(Error::MissingBlankLine);
        let request = Request::from_string("GET / HTTP/1.1");
        assert_eq!(expected, request);
    }

    #[test]
    fn new() {
        let expected = Request {
            method : Method::GET,
            version : Version::V1_1,
            path : "/".to_string(),
            headers: None,
        };
        let request = Request::from_string("GET / HTTP/1.1\r\n\r\n").expect("Error Parsing");
        assert_eq!(expected, request);
    }

    #[test]
    fn new_headers() {
        let expected = Request {
            method : Method::GET,
            version : Version::V1_1,
            path : "/".to_string(),
            headers: Some(vec!["Header1: hi".to_string(), "Header2: Bye".to_string()]),
        };
        let request = Request::from_string("GET / HTTP/1.1\r\nHeader1: hi\r\nHeader2: Bye\r\n\r\n").expect("Error Parsing");
        assert_eq!(expected, request);
    }

    #[test]
    fn empty_string() {
        let request = Request::from_string("");
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
