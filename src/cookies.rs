use crate::{
    http::{Header, IntoHeader},
    utils::{self, base64_decode, base64_encode},
};
use anyhow::Context;
use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{collections::HashMap, format, vec};

#[derive(Debug, Clone)]
pub struct CookieConfig {
    secure: bool,
    http_only: bool,
    same_site: Option<String>,
    domain: Option<String>,
    path: Option<String>,
    expiration: Option<String>, //datetime string
    secret: SecretString,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Cookie {
    config: CookieConfig,
    name: String,
    value: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CookiePayload {
    value: String,
    signature: Vec<u8>,
}

type HmacSha256 = Hmac<Sha256>;

impl PartialEq for CookieConfig {
    fn eq(&self, other: &Self) -> bool {
        self.secure == other.secure
            && self.http_only == other.http_only
            && self.same_site == other.same_site
            && self.domain == other.domain
            && self.path == other.path
            && self.expiration == other.expiration
            && self.secret.expose_secret() == other.secret.expose_secret()
    }
}

impl Eq for CookieConfig {}

/// http cookie, can be converted into a header
impl Cookie {
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

    /// Signs value of cookie and returns struct containing value and signature
    pub fn sign(&self) -> CookiePayload {
        let mut mac =
            HmacSha256::new_from_slice(self.config.secret.expose_secret().as_bytes()).unwrap();
        mac.update(self.value.as_bytes());
        let sig = mac.finalize().into_bytes().to_vec();
        CookiePayload {
            value: self.value.to_string(),
            signature: sig,
        }
    }

    pub fn value(&self) -> &str {
        self.value.as_str()
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}

/// Settings for Cookie, cookies can be built from this
/// a cookie config will also generate a random key for signing
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
            secret: utils::generate_random_secret(),
        }
    }

    pub fn new_cookie(&self, name: &str, value: &str) -> Cookie {
        Cookie::new_with_config(self, name, value)
    }

    pub fn delete_cookie(&self, name: &str) -> Cookie {
        let mut config = self.clone();
        config.expiration = Some("Thu, 01 Jan 1970 00:00:00 GMT".into());
        config.new_cookie(name, "")
    }

    pub fn is_valid_signature(&self, payload: &CookiePayload) -> Result<(), anyhow::Error> {
        let mut mac = HmacSha256::new_from_slice(self.secret.expose_secret().as_bytes())
            .context("Error Creating Signature Hash")?;
        mac.update(payload.value.as_bytes());
        mac
            .verify_slice(&payload.signature)
            .context("Invalid Signature")
    }

    pub fn cookies_from_str(&self, value: &str) -> Result<HashMap<String, Cookie>, anyhow::Error> {
        let values: Vec<_> = value.split("; ").collect();
        let iterator = values.into_iter();
        let mut config = self.clone();
        let mut map = HashMap::new();
        let mut raw_cookie_list = vec![];

        for item in iterator {
            let split: Vec<_> = item.split('=').collect();
            let n = split[0];
            match n {
                "Secure" => {
                    config.secure = true;
                }
                "HttpOnly" => {
                    config.http_only = true;
                }
                "SameSite" => {
                    if split.len() > 1 {
                        config.same_site = Some(split[1].to_string());
                    }
                }
                "Domain" => {
                    if split.len() > 1 {
                        config.domain = Some(split[1].to_string());
                    }
                }
                "Path" => {
                    if split.len() > 1 {
                        config.path = Some(split[1].to_string());
                    }
                }
                "Expires" => {
                    if split.len() > 1 {
                        config.expiration = Some(split[1].to_string());
                    }
                }
                _ => {
                    if split.len() == 2 {
                        raw_cookie_list.push((n.to_string(), split[1].to_string()));
                    } else {
                        raw_cookie_list.push((n.to_string(), String::new()));
                    }
                }
            }
        }

        for (n, v) in raw_cookie_list {
            let encoded_value = v;
            if let Ok(decoded_value) = base64_decode(encoded_value) {
                if let Ok(json_string) = String::from_utf8(decoded_value) {
                    match serde_json::from_str(&json_string) {
                        Ok(payload) => {
                            if self.is_valid_signature(&payload).is_ok() {
                                let cookie = config.new_cookie(&n, &payload.value);
                                map.insert(n, cookie);
                            }
                        }
                        Err(e) => {
                            log::warn!("Cookie Serilaztion Error: {}", e.to_string());
                        }
                    }
                } else {
                    log::warn!("Got a cookie with invalid signature");
                }
            } else {
                log::warn!("Got a cookie not from us")
            }
        }
        Ok(map)
    }

    pub fn cookies_from_header(
        &self,
        header: Header,
    ) -> Result<HashMap<String, Cookie>, anyhow::Error> {
        if header.key == "set-cookie" {
            self.cookies_from_str(&header.value)
        } else {
            Err(anyhow::Error::msg("Invalid Header Name For Cookie"))
        }
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
        let cookie_value = self.sign();
        let cookie_json =
            serde_json::to_string(&cookie_value).expect("Error Serializing Cookie value"); //FIXME: How should we
                                                                                           //handle an error here ?
        let cookie_base64 = base64_encode(cookie_json.into());
        let mut header_value = format!("{}={}", self.name, cookie_base64);

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
    use std::dbg;

    use super::*;

    #[test]
    fn set_cookie_header() {
        //let expected = "set-cookie: id=hi; Secure; HttpOnly; SameSite=Strict; Path=/";
        let config = CookieConfig::default();
        let cookie = config.new_cookie("id", "hi");
        let header = cookie.into_header();
        let decoded_coookie = config.cookies_from_header(header).unwrap();
        assert_eq!(&cookie, decoded_coookie.get("id").unwrap());
    }

    #[test]
    fn cookie_builder() {
        let config = CookieConfig::default();
        let cookie = config.new_cookie("id", "hi");
        let header = cookie.into_header();
        let decoded_cookie = config.cookies_from_header(header).unwrap();
        assert_eq!(&cookie, decoded_cookie.get("id").unwrap());
    }

    #[test]
    fn cookie_delete() {
        let config = CookieConfig::default();
        let mut cookie = config.new_cookie("id", "hi");
        let header = cookie.into_header();
        let decoded_cookie = config.cookies_from_header(header).unwrap();
        assert_eq!(&cookie, decoded_cookie.get("id").unwrap());

        cookie.delete();
        let header = cookie.into_header();
        let decoded_cookie = config.cookies_from_header(header).unwrap();
        assert_eq!(&cookie, decoded_cookie.get("id").unwrap());
    }

    #[test]
    fn other_cookies() {
        let config = CookieConfig::default();
        let cookie = config.new_cookie("id", "hi");
        let header = cookie.into_header();
        let decoded_cookie = config.cookies_from_header(header.clone()).unwrap();
        assert_eq!(&cookie, decoded_cookie.get("id").expect("no cookie"));

        let cookie_str = format!("bob=; robert=bob; this_is=c2VjcmV0; {}", header.value);
        dbg!(&cookie_str);
        let cookies = config
            .cookies_from_str(&cookie_str)
            .expect("Error Parsing String into Cookies");
        dbg!(&cookies);
        assert_eq!("hi", cookies.get("id").unwrap().value());
    }
}
