use anyhow;
use get_port::tcp::TcpPort;
use get_port::{Ops, Range};
use log;
use nucleus_http::{
    request::Request,
    routes::{Route, Router},
    virtual_host::VirtualHost,
    Server,
};
use pretty_env_logger;
use std::sync::{Arc, RwLock};
use tokio;

#[derive(Debug, Clone)]
pub struct AppState {
    greeting: String,
    bye: String,
    views: Arc<RwLock<u32>>,
}

pub async fn launch_dev_server() -> u16 {
    //pretty_env_logger::init();
    let tcp_port = TcpPort::in_range(
        "127.0.0.1",
        Range {
            min: 6000,
            max: 8000,
        },
    )
    .unwrap();
    let listener_ip = format!("0.0.0.0:{}", tcp_port);
    log::info!("Listening on {listener_ip}");
    let localhost_vhost = VirtualHost::new("localhost", &listener_ip, "./");

    let state = AppState {
        greeting: "HI".to_owned(),
        bye: "Bye".to_owned(),
        views: Arc::new(RwLock::new(0)),
    };
    let mut router = Router::new(state);
    let route = Route::get("/state/*", print_greeting);
    router.add_route(route).await;
    //router.add_route(Route::get_state("/req", print_req)).await;
    router.add_route(Route::get("/hello", get)).await;
    router.add_route(Route::get_static("/", "index.html")).await;
    let mut server = Server::bind(&listener_ip, router).await.unwrap();
    server.add_virtual_host(localhost_vhost).await;
    tokio::spawn(async move { server.serve().await.expect("Server Shutdown") });
    return tcp_port;
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
