use get_port::tcp::TcpPort;
use get_port::{Ops, Range};
use nucleus_http::cookies::CookieConfig;
use nucleus_http::http;
use nucleus_http::response::Response;
use nucleus_http::{
    request::Request,
    routes::{Route, Router},
    virtual_host::VirtualHost,
    Server,
};
use std::format;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct AppState {
    greeting: String,
    bye: String,
    views: Arc<RwLock<u32>>,
    cookie_config: CookieConfig,
}

pub async fn launch_dev_server() -> u16 {
    pretty_env_logger::init();
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

    let state = AppState {
        greeting: "HI".to_owned(),
        bye: "Bye".to_owned(),
        views: Arc::new(RwLock::new(0)),
        cookie_config: CookieConfig::default(),
    };
    let mut router = Router::new(state);
    let whats_my_cookie_route = Route::get("/whats_my_cookie", whats_my_cookie);
    let cookie_route = Route::get("/get_cookie", get_cookie);
    let route = Route::get("/state/*", print_greeting);
    router.add_route(cookie_route).await;
    router.add_route(whats_my_cookie_route).await;
    router.add_route(route).await;
    //router.add_route(Route::get_state("/req", print_req)).await;
    router.add_route(Route::get("/hello", get)).await;
    router.add_route(Route::get_static("/", "index.html")).await;
    let server = Server::bind(&listener_ip, router, "./").await.unwrap();
    tokio::spawn(async move { server.serve().await.expect("Server Shutdown") });
    return tcp_port;
}

async fn whats_my_cookie(state: AppState, r: Request) -> Result<String, String> {
    if let Some(cookie_header) = r.get_header_value("Cookie") {
        log::debug!("{:#?}", cookie_header);
        if let Ok(cookies) = state.cookie_config.cookies_from_str(&cookie_header) {
            if let Some(flavor) = cookies.get("flavor") {
                return Ok(format!("You Have A {} Cookie", flavor.value()));
            }
        }
    } else {
        log::debug!("Didnt find cookie header");
    }

    Ok("you have no cookies".to_string())
}

async fn get_cookie(state: AppState, _: Request) -> Result<Response, String> {
    let mut res = Response::new(
        http::StatusCode::OK,
        "Heres a Cookie".into(),
        http::MimeType::PlainText,
    );
    let cookie = state.cookie_config.new_cookie("flavor", "chocolate chip");
    res.add_header(cookie);

    Ok(res)
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
