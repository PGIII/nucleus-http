use crate::{http::Method, request};
use std::sync::Arc;
use tokio::sync::RwLock;

type ResolveFunction = fn(&request::Request) -> String;

pub enum RouteResolver {
    Static { file_path: String },
    Function(ResolveFunction),
}

pub struct Route {
    method: Method,
    path: String,
    resolver: RouteResolver,
}

pub type Routes = Arc<RwLock<Vec<Route>>>;

pub fn new_routes() -> Routes {
    Arc::new(RwLock::new(vec![]))
}
impl Route {
    
    pub fn get(path: &str, resolve_func: ResolveFunction) -> Route {
        let method = Method::GET;
        let resolver = RouteResolver::Function(resolve_func);
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
        let resolver = RouteResolver::Static { file_path: file_path.to_string() };
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
}