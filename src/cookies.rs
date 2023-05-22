use crate::http::{Header, IntoHeader};

#[derive(Debug, Clone)]
pub struct CookieConfig {
    secure: bool,
    http_only: bool,
    same_site: Option<String>,
    domain: Option<String>,
    path: Option<String>,
    expiration: Option<String>, //datetime string
}

pub struct Cookie {
    config: CookieConfig,
    name: String,
    value: String,
}

/// http cookie, can be converted into a header
impl Cookie {
    pub fn new(name: &str, value: &str) -> Cookie {
        Self::new_with_config(&CookieConfig::default(), name, value)
    }

    pub fn new_with_config(config: &CookieConfig, name: &str, value: &str) -> Cookie {
        Cookie {
            config: config.clone(),
            name: name.into(),
            value: value.into(),
        }
    }

    pub fn delete(&mut self) {
        self.config.expiration = Some("Thu, 01 Jan 1970 00:00:00 GMT".into())
    }
}

/// Settings for Cookie, cookies can be built from this
impl CookieConfig {
    /// Default settings for cookie
    /// defaults are Strict same site, secure, http only, path = /, and no expiration
    /// Note: Domain isnt specifed since that keeps subdomains from having access
    pub fn default() -> CookieConfig {
        CookieConfig {
            secure: true,
            http_only: true,
            same_site: Some("Strict".into()),
            domain: None,
            path: Some("/".into()),
            expiration: None,
        }
    }

    pub fn new_cookie(&self, name: &str, value: &str) -> Cookie {
        Cookie::new_with_config(self, name, value)
    }

    pub fn secure(&self) -> bool {
        self.secure
    }

    pub fn set_secure(&mut self, secure: bool) {
        self.secure = secure;
    }

    pub fn http_only(&self) -> bool {
        self.http_only
    }

    pub fn set_http_only(&mut self, http_only: bool) {
        self.http_only = http_only;
    }

    pub fn same_site(&self) -> Option<&String> {
        self.same_site.as_ref()
    }

    pub fn set_same_site(&mut self, same_site: Option<String>) {
        self.same_site = same_site;
    }

    pub fn domain(&self) -> Option<&String> {
        self.domain.as_ref()
    }

    pub fn set_domain(&mut self, domain: Option<String>) {
        self.domain = domain;
    }

    pub fn set_path(&mut self, path: Option<String>) {
        self.path = path;
    }

    pub fn path(&self) -> Option<&String> {
        self.path.as_ref()
    }

    pub fn expiration(&self) -> Option<&String> {
        self.expiration.as_ref()
    }

    pub fn set_expiration(&mut self, expiration: Option<String>) {
        self.expiration = expiration;
    }
}

impl IntoHeader for Cookie {
    fn into_header(&self) -> crate::http::Header {
        let mut header_value = format!("{}={}", self.name, self.value);
        if self.config.secure {
            header_value = format!("{}; Secure", header_value);
        }

        if self.config.http_only {
            header_value = format!("{}; HttpOnly", header_value);
        }

        if let Some(ss) = &self.config.same_site {
            header_value = format!("{}; SameSite={}", header_value, ss);
        }

        if let Some(domain) = &self.config.domain {
            header_value = format!("{}; Domain={}", header_value, domain);
        }

        if let Some(p) = &self.config.path {
            header_value = format!("{}; Path={}", header_value, p);
        }

        if let Some(exp) = &self.config.expiration {
            header_value = format!("{}; Expires={}", header_value, exp);
        }

        Header::new("Set-Cookie", &header_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_cookie_header() {
        let expected = "set-cookie: id=hi; Secure; HttpOnly; SameSite=Strict; Path=/";
        let cookie = Cookie::new("id", "hi");
        let header = cookie.into_header();
        let header_string = String::from(header);
        assert_eq!(expected, header_string);
    }

    #[test]
    fn cookie_builder() {
        let config = CookieConfig::default();
        let expected = "set-cookie: id=hi; Secure; HttpOnly; SameSite=Strict; Path=/";
        let cookie = config.new_cookie("id", "hi");
        let header = cookie.into_header();
        let header_string = String::from(header);
        assert_eq!(expected, header_string);
    }

    #[test]
    fn cookie_delete() {
        let config = CookieConfig::default();
        let expected = "set-cookie: id=hi; Secure; HttpOnly; SameSite=Strict; Path=/";
        let mut cookie = config.new_cookie("id", "hi");
        let header = cookie.into_header();
        let header_string = String::from(header);
        assert_eq!(expected, header_string);

        let expected =  "set-cookie: id=hi; Secure; HttpOnly; SameSite=Strict; Path=/; Expires=Thu, 01 Jan 1970 00:00:00 GMT";
        cookie.delete();
        let header = cookie.into_header();
        let header_string = String::from(header);
        assert_eq!(expected, header_string);
    }
}
