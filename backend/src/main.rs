use actix_files::Files;
use actix_web::{App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new()
        .service(Files::new("/static", "../frontend/static").show_files_listing())
        .service(Files::new("/", "../frontend/dist").show_files_listing())
    ).bind(("127.0.0.1", 8080))?.run().await
}
