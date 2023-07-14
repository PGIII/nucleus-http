use std::path::PathBuf;

use crate::{request::Request, response::Response, routes::Router};

pub struct VirtualHost<S> {
    hostname: String,
    root_dir: PathBuf, // root dir for static files, eg. /var/www/default
    router: Router<S>,
}

impl<S> VirtualHost<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub fn new(hostname: &str, _ip: &str, root_dir: &str, router: Router<S>) -> Self {
        Self {
            hostname: hostname.to_string(),
            //ip: ip.to_string(),
            root_dir: PathBuf::from(root_dir),
            router,
        }
    }

    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    pub fn root_dir(&self) -> &PathBuf {
        &self.root_dir
    }

    pub async fn route(&self, request: &Request) -> Response {
        self.router.route(request, &self.root_dir).await
    }
}
