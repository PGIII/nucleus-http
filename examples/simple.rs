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
}

fn print_greeting(state: AppState, request: &Request) {
    println!("{} {} and {}", state.greeting, request.path(), state.bye);
}

fn print_req(request: &Request) {
    println!("{}", request.path());
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
    };
    let mut router = Router::new(state);
    router.add_route(Route::get_state("/state", print_greeting)).await;
    //router.add_route(Route::get_state("/req", print_req)).await;
    router.add_route(Route::get_async("/async", Box::new(async_get))).await;
    router.add_route(Route::get("/sync", get)).await;
    router.add_route(Route::get_static("/", "index.html")).await;
    let mut server = Server::bind(listener_ip, router).await?;
    server.add_virtual_host(localhost_vhost).await;

    server.serve().await.unwrap();
    return Ok(());
}

fn async_get(_req: &Request) -> BoxedFuture<String> {
    Box::pin(async move { "Hello From Rust Routes!".to_string() })
}

fn get(_req: &Request) -> String {
    "Hello From Sync Func".to_string()
}
