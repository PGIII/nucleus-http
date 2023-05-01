use crate::{http::Method, request::{self, Request}};
use std::{future::Future, pin::Pin, sync::Arc, collections::HashMap};
use tokio::sync::RwLock;

pub type ResolveFunction = fn(&request::Request) -> String;
pub type BoxedFuture<T = ()> = Pin<Box<dyn Future<Output = T> + Send>>;
pub type ResolveAsyncFunction = Box<dyn Fn(&request::Request) -> BoxedFuture<String> + Send + Sync>;

pub enum RouteResolver {
    Static { file_path: String },
    AsyncFunction(ResolveAsyncFunction),
    Function(ResolveFunction),
    RedirectAll(String),
}

pub struct Route {
    method: Method,
    path: String,
    resolver: RouteResolver,
}

pub type Routes = Arc<RwLock<HashMap<String,Route>>>;

pub fn new_routes() -> Routes {
    Arc::new(RwLock::new(HashMap::new()))
}

impl Route {
    pub fn redirect_all(redirect_url: &str) -> Route {
        let method = Method::GET;
        Route {
            path: "*".to_string(),
            resolver: RouteResolver::RedirectAll(redirect_url.to_string()),
            method
        }
    }

    pub fn get(path: &str, resolve_func: ResolveFunction) -> Route {
        let method = Method::GET;
        let resolver = RouteResolver::Function(resolve_func);
        Route {
            path: path.to_string(),
            resolver,
            method,
        }
    }

    pub fn get_async(path: &str, resolve_func: ResolveAsyncFunction) -> Route {
        let method = Method::GET;
        let resolver = RouteResolver::AsyncFunction(resolve_func);
        Route {
            path: path.to_string(),
            resolver,
            method,
        }
    }

    /// Static file map
    /// Allows remapping a route to a file
    /// {file_path} is a is relative path to static file (without leading /) that will be joined
    /// with vhost root dir to serve
    /// eg. path = / file_path = index.html will remap all "/" requests to index.html
    pub fn get_static(path: &str, file_path: &str) -> Route {
        let method = Method::GET;
        let resolver = RouteResolver::Static {
            file_path: file_path.to_string(),
        };
        Route {
            path: path.to_string(),
            resolver,
            method,
        }
    }

    pub fn method(&self) -> &Method {
        return &self.method;
    }

    pub fn resolver(&self) -> &RouteResolver {
        return &self.resolver;
    }

    pub fn path(&self) -> &str {
        return &self.path;
    }

    ///FIXME: this should be a little more robust and look for wild cards only if not route is
    ///defined.
    ///as well as look for redirect all paths first and default to them
    ///for now if you want redirect all that should be the only route on the server
    pub fn matches_request(&self, request: &Request) -> bool {
        if request.method() != self.method() {
            return false;
        }

        // Check for exact match or if route is wild card
        let request_path = request.path(); 
        let route_path = self.path();
        let path_match = request_path == route_path || route_path == "*";

        return path_match;
    }
}
