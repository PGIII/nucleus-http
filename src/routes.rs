use crate::{
    http::{Method, StatusCode},
    request,
    request::Request,
    response::Response,
    virtual_host::VirtualHost,
};
use tokio::fs;

type ResolveFunction = fn(&request::Request) -> String;
pub enum RouteResolver {
    Static { file_path: String },
    Function(ResolveFunction),
}

pub struct Route {
    vhost: Option<VirtualHost>, // no Vhost means it applies to all vhost
    method: Method,
    path: String,
    resolver: RouteResolver,
}

//FIXME: this can probably be faster with a hash table
pub struct Routes {
    get: Vec<Route>,
    post: Vec<Route>,
}

impl Route {
    //FIXME: combine these two using generics
    pub fn get(path: String, resolve_func: ResolveFunction, vhost: Option<VirtualHost>) -> Route {
        let method = Method::GET;
        let resolver = RouteResolver::Function(resolve_func);
        let vhost = vhost;
        Route {
            path,
            resolver,
            method,
            vhost,
        }
    }

    pub fn get_static(path: String, file_path: String, vhost: Option<VirtualHost>) -> Route {
        let method = Method::GET;
        let resolver = RouteResolver::Static { file_path };
        let vhost = vhost;
        Route {
            path,
            resolver,
            method,
            vhost,
        }
    }

    pub fn vhost(&self) -> &Option<VirtualHost> {
        return &self.vhost;
    }
}

impl Routes {
    pub fn new() -> Routes {
        Routes {
            get: vec![],
            post: vec![],
        }
    }

    pub fn add(&mut self, route: Route) {
        match route.method {
            Method::GET => {
                self.get.push(route);
            }
            Method::POST => {
                self.post.push(route);
            }
        }
    }

    /// runs router returning a response that should be forwarded to client
    pub async fn run(&self, request: &request::Request) -> Response {
        match request.method() {
            Method::GET => {
                for route in &self.get {
                    if Self::routes_request_match(request, &route) {
                        match &route.resolver {
                            RouteResolver::Static { file_path } => {
                                let response;
                                if let Ok(contents) = fs::read_to_string(file_path).await {
                                    response = Response::from(contents);
                                } else {
                                    response = Response::error(
                                        StatusCode::ErrNotFound,
                                        "File Not Found".to_string(),
                                    );
                                }
                                return response;
                            }
                            RouteResolver::Function(func) => {
                                let func_return = func(&request);
                                return Response::from(func_return);
                            }
                        }
                    }
                }
                let response = Response::error(StatusCode::ErrNotFound, "File Not Found".to_string());
                return response;
            }
            Method::POST => {
                todo!();
            }
        }
    }

    fn routes_request_match(request: &Request, route: &Route) -> bool {
        let path_match = request.path() == route.path;
        let host_match;
        if let Some(vhost) = route.vhost() {
            host_match = request.hostname() == vhost.hostname();
        } else {
            host_match = true;
        }
        return path_match && host_match;
    }
}
