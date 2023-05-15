use crate::{
    http::{self, Method, MimeType},
    request::Request,
    response::{IntoResponse, Response},
    state::{FromRequest, State},
    Connection,
};
use async_trait::async_trait;
use std::{
    collections::HashMap,
    future::Future,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::RwLock;

#[async_trait]
pub trait RequestResolver<S>: Send + Sync + 'static {
    async fn resolve(&self, state: State<S>, request: Request) -> Response;
}

#[async_trait]
impl<F, P, O, E, Fut> RequestResolver<P> for F
where
    O: IntoResponse,
    E: IntoResponse,
    Fut: Future<Output = Result<O, E>> + Send + 'static,
    F: Fn(P, Request) -> Fut + Send + Sync + 'static,
    P: FromRequest<P> + Send + Sync + 'static,
{
    async fn resolve(&self, state: State<P>, request: Request) -> Response {
        let result = (self)(P::from_request(state, request.clone()), request).await;
        match result {
            Ok(r) => r.into_response(),
            Err(e) => e.into_response(),
        }
    }
}

pub enum RouteResolver<S> {
    Static { file_path: String },
    RedirectAll(String),
    Function(Arc<Box<dyn RequestResolver<S>>>),
}

pub struct Route<S> {
    method: Method,
    path: String,
    resolver: RouteResolver<S>,
}

pub type Routes<R> = Arc<RwLock<HashMap<String, Route<R>>>>;

#[derive(Clone)]
pub struct Router<S> {
    routes: Routes<S>,
    state: State<S>,
}

impl<S> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub fn new(state: S) -> Self {
        let routes = HashMap::new();
        Router {
            routes: Arc::new(RwLock::new(routes)),
            state: State(state),
        }
    }

    pub async fn add_route(&mut self, route: Route<S>) {
        let mut routes_locked = self.routes.write().await;
        routes_locked.insert(route.path.clone(), route);
    }

    pub fn routes(&self) -> Routes<S> {
        return Arc::clone(&self.routes);
    }

    pub fn new_routes() -> Routes<S> {
        Arc::new(RwLock::new(HashMap::new()))
    }

    pub async fn route(&self, request: &Request, connection: &Connection) -> Response {
        log::info!("{} Request for: {}", request.method(), request.path());
        let routes = self.routes();
        let routes_locked = routes.read().await;
        let mut matching_route = None;

        //look for route mathcing requested URL
        if let Some(route) = routes_locked.get(request.path()) {
            //found exact route match
            matching_route = Some(route);
        } else {
            // go through ancestors appending * on the end and see if we have any matches
            let path = Path::new(request.path());
            if let Some(parent) = path.parent() {
                let mut ancestors = parent.ancestors();
                while let Some(a) = ancestors.next() {
                    log::debug!("checking ancestor: {}", a.to_string_lossy());
                    if let Some(globed) = a.join("*").to_str() {
                        if let Some(route) = routes_locked.get(globed) {
                            matching_route = Some(route);
                        }
                    }
                }
            }
        }

        //serve specific route if we match
        if let Some(route) = matching_route {
            match route.resolver() {
                RouteResolver::Static { file_path } => {
                    if let Some(host_dir) = Self::get_vhost_dir(request, connection).await {
                        let path = host_dir.join(file_path);
                        return Self::get_file(path).await;
                    }
                }
                RouteResolver::RedirectAll(redirect_to) => {
                    let mut response = Response::new(
                        http::StatusCode::MovedPermanetly,
                        vec![],
                        MimeType::PlainText,
                    );
                    response.add_header("Location", &redirect_to);
                    return response;
                }
                RouteResolver::Function(resolver) => {
                    let resolver = resolver.clone();
                    return resolver
                        .resolve(self.state.clone(), request.to_owned())
                        .await;
                }
            }
        }

        //server static files based on vhost
        if let Some(host_dir) = Self::get_vhost_dir(request, connection).await {
            let mut file_path = PathBuf::from(request.path());
            if file_path.is_absolute() {
                if let Ok(path) = file_path.strip_prefix("/") {
                    file_path = path.to_path_buf();
                } else {
                    return Response::error(http::StatusCode::ErrNotFound, "File Not Found".into());
                }
            }
            let final_path = host_dir.join(file_path);
            return Self::get_file(final_path).await;
        }

        //no route try static serve
        let response = Response::error(http::StatusCode::ErrNotFound, "File Not Found".into());
        return response;
    }

    async fn get_file(path: PathBuf) -> Response {
        match tokio::fs::read(&path).await {
            Ok(contents) => {
                let mime: MimeType = path.into();
                let response = Response::new(http::StatusCode::OK, contents, mime);
                return response;
            }
            Err(err) => match err.kind() {
                std::io::ErrorKind::PermissionDenied => {
                    let response =
                        Response::error(http::StatusCode::ErrForbidden, "Permission Denied".into());
                    return response;
                }
                std::io::ErrorKind::NotFound | _ => {
                    let response = Response::error(
                        http::StatusCode::ErrNotFound,
                        "Static File Not Found".into(),
                    );
                    return response;
                }
            },
        }
    }
    async fn get_vhost_dir(request: &Request, connection: &Connection) -> Option<PathBuf> {
        for vhost in &*connection.virtual_hosts().read().await {
            if vhost.hostname() == request.hostname() {
                return Some(vhost.root_dir().to_path_buf());
            }
        }
        return None;
    }
}

impl<S> Route<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub fn redirect_all(redirect_url: &str) -> Self {
        let method = Method::GET;
        Route {
            path: "*".to_string(),
            resolver: RouteResolver::RedirectAll(redirect_url.to_string()),
            method,
        }
    }

    pub fn get<R>(path: &str, func: R) -> Self 
    where 
        R: RequestResolver<S>
    {
        let method = Method::GET;
        let resolver = RouteResolver::Function(Arc::new(Box::new(func)));
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
    pub fn get_static(path: &str, file_path: &str) -> Self {
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

    pub fn resolver(&self) -> &RouteResolver<S> {
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
