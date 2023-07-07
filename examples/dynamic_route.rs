use nucleus_http::{
    request::Request,
    routes::{Route, Router},
    Server,
};

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    pretty_env_logger::init();
    let listener_ip = "0.0.0.0:7878";
    log::info!("Listening on {listener_ip}");
    let mut router = Router::new(());
    router
        .add_route(Route::get("/async", Box::new(async_get)))
        .await;
    router.add_route(Route::get("/sync", Box::new(get))).await;
    //match on all hi/ routes
    router
        .add_route(Route::get("/hi/*", Box::new(dynamic_hello)))
        .await;
    router.add_route(Route::get_static("/", "index.html")).await;
    let server = Server::bind(listener_ip, router, "./").await?;
    server.serve().await.unwrap();
    Ok(())
}

async fn async_get(_: (), _: Request) -> Result<String, String> {
    Ok("Hello From Rust Routes!".to_string())
}

async fn get(_: (), _: Request) -> Result<String, String> {
    Ok("Hello From Sync Func".to_string())
}

async fn dynamic_hello(_: (), req: Request) -> Result<String, String> {
    Ok(format!("Hello from URL: {}", req.path()))
}
