use crate::{request, request::Method, methods};

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

//FIXME: this can probably be faster with a hash table
pub struct Routes {
    get: Vec<Route>,
    post: Vec<Route>,
}

impl Route {
    //FIXME: combine these two using generics
    pub fn get(path: String, resolve_func: ResolveFunction) -> Route {
        let method = Method::GET;
        let resolver = RouteResolver::Function(resolve_func);
        Route { path, resolver, method }
    }

    pub fn get_static(path: String, file_path: String) -> Route {
        let method = Method::GET;
        let resolver = RouteResolver::Static{file_path};
        Route { path, resolver, method }
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
    pub async fn run(&self, request: &request::Request) -> String {
        let path: &str = request.path();
        match request.method() {
            Method::GET => {
                for route in &self.get {
                    if route.path == path {
                        println!("Found Get Route: {0}", path);
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
}
