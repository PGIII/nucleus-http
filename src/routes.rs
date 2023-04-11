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
    //FIXME: combine these two using generics
    pub fn get(path: String, resolve_func: ResolveFunction) -> Route {
        let method = Method::GET;
        let resolver = RouteResolver::Function(resolve_func);
        Route {
            path,
            resolver,
            method,
        }
    }

    pub fn get_static(path: String, file_path: String) -> Route {
        let method = Method::GET;
        let resolver = RouteResolver::Static { file_path };
        Route {
            path,
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
