use std::path::{PathBuf, Path};

pub struct VirtualHost {
    hostname: String, 
    ip: String,
    root_dir: PathBuf, // root dir for static files, eg. /var/www/default
}

impl VirtualHost {
    pub fn new(hostname: &str, ip: &str, root_dir: &str) -> VirtualHost {
        let vhost = VirtualHost {
            hostname: hostname.to_string(),
            ip: ip.to_string(),
            root_dir: PathBuf::from(root_dir),
        };
        return vhost;
    }
    pub fn hostname(&self) -> &str {
        &self.hostname
    }
}
