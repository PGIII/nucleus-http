use crate::{request, request::{Method, Request}, methods, virtual_host::VirtualHost};

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
        Route { path, resolver, method, vhost}
    }

    pub fn get_static(path: String, file_path: String, vhost: Option<VirtualHost>) -> Route {
        let method = Method::GET;
        let resolver = RouteResolver::Static{file_path};
        let vhost = vhost;
        Route { path, resolver, method, vhost}
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
            },
            Method::POST => {
                self.post.push(route);
            }
        }
    }    

    /// runs router returning a response that should be forwarded to client
    pub async fn run(&self, request: &request::Request) -> String {
        let path: &str = request.path();
        match request.method() {
            Method::GET => {
                for route in &self.get {
                    if Self::routes_request_match(request, &route) {
                        match &route.resolver {
                            RouteResolver::Static { file_path } => {
                                return methods::get::load_file(request, file_path).await;
                            }
                            RouteResolver::Function(func) => {
                                return func(request);
                            }
                        }
                    }
                }
                return methods::get::response("<h1>404 File Not Found</h1>".to_string(), request.error(404, "Not Found"));
            },
            Method::POST => {
                return "IMPLEMENT ME".to_owned();
            }
        }
    }
    fn routes_request_match(request: &Request, route: &Route) -> bool {
        let path_match = request.path() == route.path;
        return path_match;  
    }
}
