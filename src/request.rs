#[derive(PartialEq, Debug)]
pub struct Request {
    method: Method,
    path: String,
    version: Version,
    headers: Option<String>,
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
}

impl Request {
    pub fn from_string(request_str: &str) -> Result<Request, Error> {
        //Sepperate String based on spaces, first seperated string should be method
        if request_str.len() == 0 {
            return Err(Error::InvalidString);
        }

        let method;
        let version;
        let path;
        let mut headers = None;
        let space_seperate: Vec<&str> = request_str.split(" ").collect();
        let string_count = space_seperate.len();
        if  string_count < 3 {
            return Err(Error::InvalidString);
        }

        //First is method
        match space_seperate[0] {
            "GET" => method = Method::GET,
            "POST" => method = Method::POST,
            _ => return Err(Error::InvalidMethod),
        }

        //second string is url
        path = space_seperate[1].to_string();

        //third is http Verison
        match space_seperate[2] {
            "HTTP/1.1" => version = Version::V1_1,
            "HTTP/2.2" => version = Version::V2_0,
            _ => return Err(Error::InvalidHTTPVersion),
        }

        //4th is optional headers
        if string_count > 4 {
            //FIXME: Dont we need to collect here?
            headers = Some(space_seperate[4].to_string());
        }

        //last is optional headers
        return Ok(Request { 
            method,
            version,
            path,
            headers,
        })      
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_wrong_version_new() {
        let expected = Err(Error::InvalidHTTPVersion);
        let request = Request::from_string("GET / HTTP1.1");
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
        let request = Request::from_string("GET / HTTP/1.1").expect("Error Parsing");
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
