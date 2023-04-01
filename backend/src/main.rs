use actix_files::{Files, NamedFile};
use actix_web::{web, App, HttpServer, Result};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(Files::new("/static", "../frontend/static").show_files_listing())
            .service(Files::new("/", "../frontend/dist").index_file("index.html"))
            .default_service(web::route().to(index))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

async fn index() -> Result<NamedFile> {
    Ok(NamedFile::open("../frontend/dist/index.html")?)
}
