use std::sync::{Arc, RwLock};

use anyhow;
use log;
use nucleus_http::{
    request::Request,
    routes::{Route, Router},
    virtual_host::VirtualHost,
    Server,
};
use pretty_env_logger;
use tokio;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Debug, Clone)]
struct AppState {
    greeting: String,
    bye: String,
    views: Arc<RwLock<u32>>,
}

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_thread_names(true)
        .pretty();
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("debug"))
        .unwrap();
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();
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
        views: Arc::new(RwLock::new(0)),
    };
    let mut router = Router::new(state);
    let route = Route::get("/state", print_greeting);
    router.add_route(route).await;
    //router.add_route(Route::get_state("/req", print_req)).await;
    router.add_route(Route::get("/hello", get)).await;
    router.add_route(Route::get_static("/", "index.html")).await;
    let mut server = Server::bind(listener_ip, router).await?;
    server.add_virtual_host(localhost_vhost).await;

    server.serve().await.unwrap();
    return Ok(());
}

async fn print_greeting(state: AppState, request: Request) -> Result<String, String> {
    let views = state.views.clone();
    let mut views_write = views.write().unwrap();
    let response = format!(
        "{} {} and {}, Viewed: {}",
        state.greeting,
        request.path(),
        state.bye,
        views_write
    );
    *views_write = *views_write + 1;
    drop(views_write);
    return Ok(response);
}

async fn get(_: AppState, _: Request) -> Result<String, anyhow::Error> {
    Ok("Hello From Sync Func".to_string())
}
