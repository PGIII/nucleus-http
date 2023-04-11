use std::path::PathBuf;

pub struct VirtualHost {
    hostname: String, 
    //ip: String,
    root_dir: PathBuf, // root dir for static files, eg. /var/www/default
}

impl VirtualHost {
    pub fn new(hostname: &str, _ip: &str, root_dir: &str) -> VirtualHost {
        let vhost = VirtualHost {
            hostname: hostname.to_string(),
            //ip: ip.to_string(),
            root_dir: PathBuf::from(root_dir),
        };
        return vhost;
    }

    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    pub fn root_dir(&self) -> &PathBuf {
        &self.root_dir
    }
}
