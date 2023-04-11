use crate::{http::Method, request, virtual_host::VirtualHost};

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
