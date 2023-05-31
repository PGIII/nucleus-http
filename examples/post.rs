use std::{convert::Infallible, sync::Arc};
use tokio::sync::RwLock;

use log;
use nucleus_http::{
    http,
    request::{FormTypes, Request},
    response::Response,
    routes::{Route, Router},
    virtual_host::VirtualHost,
    Server,
};
use pretty_env_logger;
use tokio;

#[derive(Debug, Clone)]
struct AppState {
    greeting: String,
    bye: String,
    views: Arc<RwLock<u32>>,
    first_name: Arc<RwLock<String>>,
    last_name: Arc<RwLock<String>>,
}

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    pretty_env_logger::init();
    let listener_ip = "0.0.0.0:7878";
    log::info!("Listening on {listener_ip}");
    let localhost_vhost = VirtualHost::new("localhost", "0.0.0.0:7878", "./");

    let state = AppState {
        greeting: "HI".to_owned(),
        bye: "Bye".to_owned(),
        first_name: Arc::new(RwLock::new("".to_owned())),
        last_name: Arc::new(RwLock::new("".to_owned())),
        views: Arc::new(RwLock::new(0)),
    };
    let mut router = Router::new(state);
    let route = Route::get("/state", print_greeting);
    router.add_route(route).await;
    //router.add_route(Route::get_state("/req", print_req)).await;
    router.add_route(Route::get("/hello", get)).await;
    router.add_route(Route::get_static("/", "forms.html")).await;
    router.add_route(Route::get("/success", success)).await;
    router.add_route(Route::post("/handle_form", post)).await;
    router
        .add_route(Route::post("/multipart_form", multipart))
        .await;

    let mut server = Server::bind(listener_ip, router).await?;
    server.add_virtual_host(localhost_vhost).await;

    server.serve().await.unwrap();
    return Ok(());
}

async fn print_greeting(state: AppState, request: Request) -> Result<String, String> {
    let views = state.views.clone();
    let mut views_write = views.write().await;
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

async fn get(_: AppState, _: Request) -> Result<String, String> {
    Ok("Hello From Sync Func".to_string())
}

async fn success(state: AppState, _: Request) -> Result<String, Infallible> {
    let last = state.last_name.clone();
    let last = last.read().await;
    let first = state.first_name.clone();
    let first = first.read().await;
    Ok(format!("Hello {} {}", *first, *last))
}

async fn post(state: AppState, req: Request) -> Result<Response, Infallible> {
    let mut res = Response::new(
        http::StatusCode::Found,
        "".into(),
        http::MimeType::PlainText,
    );
    let body_string = String::from_utf8_lossy(req.body());
    let params: Vec<_> = body_string.split("&").collect();
    for param in params {
        let split: Vec<_> = param.split("=").collect();
        if split[0] == "fname" {
            let name = state.first_name.clone();
            let mut name_locked = name.write().await;
            *name_locked = split[1].to_string();
        } else if split[0] == "lname" {
            let name = state.last_name.clone();
            let mut name_locked = name.write().await;
            *name_locked = split[1].to_string();
        }
    }
    res.add_header(("Location", "/success"));
    Ok(res)
}

async fn multipart(state: AppState, req: Request) -> Result<Response, Infallible> {
    match req.form_data() {
        FormTypes::MultiPart(f) => {
            if let Some(file_entry) = f.get("cover_image") {
                return Ok(format!("Got File: {}", file_entry.file_name().unwrap()).into());
            }
        }
        _ => {
            return Ok("Something went wrong".into());
        }
    }

    Ok("Something went wrong".into())
}
