use std::sync::{RwLock, Arc};

use nucleus_http::{
    request::Request,
    routes::{BoxedFuture, Route, Router},
    virtual_host::VirtualHost,
    Server,
};
use tokio;
use pretty_env_logger;
use log;

#[derive(Debug, Clone)]
struct AppState {
    greeting: String,
    bye: String,
    views: Arc<RwLock<u32>>,
}


#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    pretty_env_logger::init();
    let listener_ip = "0.0.0.0:7878";
    log::info!("Listening on {listener_ip}");
    let localhost_vhost = VirtualHost::new(
        "localhost",
        "0.0.0.0:7878",
        "/Users/prestongarrisoniii/dev/source/nucleus-http/",
    );
    
    let state = AppState {
        greeting: "HI".to_owned(),
        bye: "Bye".to_owned(),
        views: Arc::new(RwLock::new(0))
    };
    let mut router = Router::new(state);
    router.add_route(Route::get_state("/state", print_greeting)).await;
    //router.add_route(Route::get_state("/req", print_req)).await;
    router.add_route(Route::get_async("/async", Box::new(async_get))).await;
    router.add_route(Route::get_state("/sync", get)).await;
    router.add_route(Route::get_static("/", "index.html")).await;
    let mut server = Server::bind(listener_ip, router).await?;
    server.add_virtual_host(localhost_vhost).await;

    server.serve().await.unwrap();
    return Ok(());
}

fn print_greeting(state: AppState, request: &Request) -> String {
    let views = state.views.clone();
    let mut views_write = views.write().unwrap();
    let response = format!("{} {} and {}, Viewed: {}", state.greeting, request.path(), state.bye, views_write);
    *views_write = *views_write + 1;
    drop(views_write);
    return response;
}

fn async_get(_req: &Request) -> BoxedFuture<String> {
    Box::pin(async move { "Hello From Rust Routes!".to_string() })
}

fn get(_: AppState, request: &Request) -> String {
    "Hello From Sync Func".to_string()
}
