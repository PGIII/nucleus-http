# nucleus-http
Rust Web frame work

## Basic Example With State and Serving Static Files
``` rust
use nucleus_http::{
    request::Request,
    routes::{Route, Router},
    Server,
};
use std::sync::{Arc, RwLock};
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

    let state = AppState {
        greeting: "HI".to_owned(),
        bye: "Bye".to_owned(),
        views: Arc::new(RwLock::new(0)),
    };
    let mut router = Router::new(state);
    let route = Route::get("/state", print_greeting);
    router.add_route(route).await;
    router.add_route(Route::get("/hello", get)).await;
    router.add_route(Route::get_static("/", "index.html")).await;
    let server = Server::bind(listener_ip, router.clone(), "./").await?;
    server.serve().await.unwrap();
    Ok(())
}

async fn print_greeting(state: AppState, request: Request) -> Result<String, String> {
    let mut views_write = state.views.write().unwrap();
    let response = format!(
        "{} {} and {}, Viewed: {}",
        state.greeting,
        request.path(),
        state.bye,
        views_write
    );
    *views_write += 1;
    Ok(response)
}

async fn get(_: AppState, _: Request) -> Result<String, anyhow::Error> {
    Ok("Hello From Sync Func".to_string())
}
```
