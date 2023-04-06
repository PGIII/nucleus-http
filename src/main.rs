use rust_web_server::{request::Request, Server, routes::Route, methods};
use tokio;

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let listener_ip = "0.0.0.0:7878";
    println!("Listening on {listener_ip}");
    let mut server = Server::bind(listener_ip).await?;
    server.add_route(Route::get("/".to_owned(), base_get)).await;
    let result = server.serve().await;
    return result;
}

fn base_get(req: &Request) -> String {
    let body = "Hello From Rust Routes!".to_owned();
    let status = req.ok();
    let response = methods::get::response(body, status);
    return response;
}
