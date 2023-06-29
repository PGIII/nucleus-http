use crate::{
    http::{self, Method, MimeType},
    request::Request,
    response::{IntoResponse, Response},
    state::{FromRequest, State},
    virtual_host::VirtualHost,
};
use async_trait::async_trait;
use enum_map::{enum_map, EnumMap};
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
    Redirect(String),
    Function(Arc<Box<dyn RequestResolver<S>>>),
    Embed(&'static [u8], MimeType),
}

pub struct Route<S> {
    method: Method,
    path: String,
    resolver: RouteResolver<S>,
}

pub type Routes<R> = Arc<RwLock<EnumMap<Method, HashMap<String, Route<R>>>>>;

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
        let map = enum_map! {
            crate::routes::Method::GET | crate::routes::Method::POST => HashMap::new(),
        };
        Router {
            routes: Arc::new(RwLock::new(map)),
            state: State(state),
        }
    }

    pub async fn add_route(&mut self, route: Route<S>) {
        let mut routes_locked = self.routes.write().await;
        routes_locked[*route.method()].insert(route.path.clone(), route);
    }

    #[tracing::instrument(level = "debug", skip(self))]
    pub fn routes(&self) -> Routes<S> {
        Arc::clone(&self.routes)
    }

    pub fn new_routes() -> Routes<S> {
        let map = enum_map! {
            crate::routes::Method::GET | crate::routes::Method::POST => HashMap::new(),
        };
        Arc::new(RwLock::new(map))
    }

    #[tracing::instrument(level = "debug", skip(self, vhosts))]
    pub async fn route(
        &self,
        request: &Request,
        vhosts: Arc<RwLock<Vec<VirtualHost>>>,
    ) -> Response {
        let routes = self.routes();
        let routes_locked = &routes.read().await[*request.method()];
        let mut matching_route = None;

        //look for route mathcing requested URL
        if let Some(route) = routes_locked.get(request.path()) {
            //found exact route match
            matching_route = Some(route);
        } else {
            // go through ancestors appending * on the end and see if we have any matches
            let path = Path::new(request.path());
            if let Some(parent) = path.parent() {
                let ancestors = parent.ancestors();
                for a in ancestors {
                    if let Some(globed) = a.join("*").to_str() {
                        if let Some(route) = routes_locked.get(globed) {
                            matching_route = Some(route);
                        }
                    }
                }
            } else {
                //no parent so its root, check for catch all bare *
                if let Some(route) = routes_locked.get("*") {
                    matching_route = Some(route);
                }
            }
        }

        //serve specific route if we match
        if let Some(route) = matching_route {
            tracing::debug!("Found matching route");
            match route.resolver() {
                RouteResolver::Static { file_path } => {
                    if let Some(host_dir) = Self::get_vhost_dir(request, vhosts.clone()).await {
                        let path = host_dir.join(file_path);
                        Self::get_file(path).await
                    } else {
                        Response::error(http::StatusCode::ErrNotFound, "File Not Found".into())
                    }
                }
                RouteResolver::Redirect(redirect_to) => {
                    let mut response = Response::new(
                        http::StatusCode::MovedPermanetly,
                        vec![],
                        MimeType::PlainText,
                    );
                    response.add_header(("Location", redirect_to));
                    response
                }
                RouteResolver::Function(resolver) => {
                    let resolver = resolver.clone();
                    resolver
                        .resolve(self.state.clone(), request.to_owned())
                        .await
                }
                RouteResolver::Embed(body, mime_type) => {
                    Response::new(http::StatusCode::OK, body.to_vec(), *mime_type)
                }
            }
        } else if let Some(host_dir) = Self::get_vhost_dir(request, vhosts).await {
            tracing::debug!("Trying static file serve");
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
        } else {
            //no route try static serve
            Response::error(http::StatusCode::ErrNotFound, "File Not Found".into())
        }
    }

    #[tracing::instrument(level = "debug")]
    async fn get_file(path: PathBuf) -> Response {
        match tokio::fs::read(&path).await {
            Ok(contents) => {
                let mime: MimeType = path.into();
                Response::new(http::StatusCode::OK, contents, mime)
            }
            Err(err) => {
                tracing::warn!("static load error:{}", err.to_string());
                match err.kind() {
                    std::io::ErrorKind::PermissionDenied => {
                        Response::error(http::StatusCode::ErrForbidden, "Permission Denied".into())
                    }
                    _ => Response::error(
                        http::StatusCode::ErrNotFound,
                        "Static File Not Found".into(),
                    ),
                }
            }
        }
    }
    async fn get_vhost_dir(
        request: &Request,
        vhosts: Arc<RwLock<Vec<VirtualHost>>>,
    ) -> Option<PathBuf> {
        for vhost in &*vhosts.read().await {
            if vhost.hostname() == request.hostname() {
                return Some(vhost.root_dir().to_path_buf());
            }
        }
        None
    }
}

impl<S> Route<S>
where
    S: Clone + Send + Sync + 'static,
{
    /// Route that redirects to another URL
    pub fn redirect(path: &str, redirect_url: &str) -> Self {
        let method = Method::GET;
        Route {
            path: path.to_string(),
            resolver: RouteResolver::Redirect(redirect_url.to_string()),
            method,
        }
    }

    /// Reroutes all traffic to url
    pub fn redirect_all(redirect_url: &str) -> Self {
        let method = Method::GET;
        Route {
            path: "*".to_string(),
            resolver: RouteResolver::Redirect(redirect_url.to_string()),
            method,
        }
    }

    pub fn get<R>(path: &str, func: R) -> Self
    where
        R: RequestResolver<S>,
    {
        let method = Method::GET;
        let resolver = RouteResolver::Function(Arc::new(Box::new(func)));
        Route {
            path: path.to_string(),
            resolver,
            method,
        }
    }

    pub fn post<R>(path: &str, func: R) -> Self
    where
        R: RequestResolver<S>,
    {
        let method = Method::POST;
        let resolver = RouteResolver::Function(Arc::new(Box::new(func)));
        Route {
            path: path.to_string(),
            resolver,
            method,
        }
    }

    /// use include_bytes! to load a file as static
    /// when this route is requested the static data is return with the passes mime type
    pub fn embed(path: &str, body: &'static [u8], mime: MimeType) -> Self {
        let method = Method::GET;
        let resolver = RouteResolver::Embed(body, mime);
        Route {
            method,
            path: path.into(),
            resolver,
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
        &self.method
    }

    pub fn resolver(&self) -> &RouteResolver<S> {
        &self.resolver
    }

    pub fn path(&self) -> &str {
        &self.path
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

        request_path == route_path || route_path == "*"
    }
}

#[macro_export]
macro_rules! embed_route {
    ($route_path:expr, $file_path:expr) => {
        //embed file
        Route::embed(
            $route_path,
            include_bytes!($file_path),
            PathBuf::from($file_path).into(),
        )
    };
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::virtual_host::VirtualHost;

    #[tokio::test]
    async fn create_embedded_html_route() {
        let route: Route<()> = embed_route!("/test", "../index.html");
        assert_eq!(route.path, "/test", "route path incorrect");
        assert_eq!(route.method, Method::GET, "route method incorrect");
        if let RouteResolver::Embed(body, mime) = route.resolver {
            assert_eq!(
                include_bytes!("../index.html"),
                body,
                "embedded body incorect"
            );
            assert_eq!(MimeType::HTML, mime);
        } else {
            panic!("wrong route type");
        }
    }

    #[tokio::test]
    async fn route_static_file() {
        let request =
            Request::from_string("GET /index.html HTTP/1.1\r\nHost: localhost\r\n\r\n".to_owned())
                .unwrap();
        let vhost = VirtualHost::new("localhost", "", "./");
        let vhosts = Arc::new(RwLock::new(vec![vhost]));
        let mut router = Router::new(());
        router.add_route(Route::get_static("/", "index.html")).await;

        let file = tokio::fs::read_to_string("./index.html").await.unwrap();
        let expected = Response::from(file);
        assert_eq!(http::StatusCode::OK, expected.status());

        let response = router.route(&request, vhosts.clone()).await;
        assert_eq!(expected, response);

        let request =
            Request::from_string("GET / HTTP/1.1\r\nHost: localhost\r\n\r\n".to_owned()).unwrap();
        let response = router.route(&request, vhosts.clone()).await;
        assert_eq!(expected, response);
    }

    async fn hello(_: (), _: Request) -> Result<String, String> {
        Ok("hello".to_owned())
    }

    #[tokio::test]
    async fn route_basic() {
        let request =
            Request::from_string("GET / HTTP/1.1\r\nHost: localhost\r\n\r\n".to_owned()).unwrap();
        let vhost = VirtualHost::new("localhost", "", "./");
        let vhosts = Arc::new(RwLock::new(vec![vhost]));
        let mut router = Router::new(());
        router.add_route(Route::get("/", hello)).await;

        let expected = Response::from("hello");
        assert_eq!(http::StatusCode::OK, expected.status());

        let response = router.route(&request, vhosts.clone()).await;
        assert_eq!(expected, response);
    }

    async fn dynamic(_: (), req: Request) -> Result<String, String> {
        Ok(format!("Hello {}", req.path()))
    }

    #[tokio::test]
    async fn route_dynamic() {
        let vhost = VirtualHost::new("localhost", "", "./");
        let vhosts = Arc::new(RwLock::new(vec![vhost]));
        let mut router = Router::new(());
        router.add_route(Route::get("/*", dynamic)).await;

        let expected = Response::from("Hello /bob");
        assert_eq!(http::StatusCode::OK, expected.status());

        let request =
            Request::from_string("GET /bob HTTP/1.1\r\nHost: localhost\r\n\r\n".to_owned())
                .unwrap();
        let response = router.route(&request, vhosts.clone()).await;
        assert_eq!(expected, response);
    }
}
