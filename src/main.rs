use rust_web_server::{request::Request, Server, routes::Route, methods, virtual_host::VirtualHost};
use tokio;

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let listener_ip = "0.0.0.0:7878";
    println!("Listening on {listener_ip}");
    let mut server = Server::bind(listener_ip).await?;
    let localhost_vhost = VirtualHost::new("localhost", "0.0.0.0:7878", "/var/www/default");
    server.add_route(Route::get_static("/".to_owned(), "/Users/prestongarrisoniii/dev/source/rust_web_server/index.html".to_string(), None)).await;
    server.add_route(Route::get("/locals".to_owned(), base_get, Some(localhost_vhost))).await;
    let result = server.serve().await;
    return result;
}

fn base_get(req: &Request) -> String {
    let body = "Hello From Rust Routes!".to_owned();
    let status = req.ok();
    let response = methods::get::response(body, status);
    return response;
}
