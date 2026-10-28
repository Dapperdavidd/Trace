pub mod config;
pub mod error;
pub mod request_id;
pub mod routes;
pub mod state;

use actix_web::{web, App, HttpServer};
use std::net::TcpListener;

pub use state::AppState;

/// Build the route table. Shared by `main` and integration tests.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1")
            .configure(routes::health::configure)
            .configure(routes::chains::configure)
            .configure(routes::openapi::configure),
    );
}

pub fn run(
    listener: TcpListener,
    state: AppState,
    cors_origins: Vec<String>,
) -> std::io::Result<actix_web::dev::Server> {
    let data = web::Data::new(state);
    let server = HttpServer::new(move || {
        let mut cors = actix_cors::Cors::default()
            .allowed_methods(["GET", "POST", "PATCH", "DELETE"])
            .allowed_headers([
                actix_web::http::header::AUTHORIZATION,
                actix_web::http::header::CONTENT_TYPE,
            ])
            .max_age(3600);
        for o in &cors_origins {
            cors = cors.allowed_origin(o);
        }
        App::new()
            .app_data(data.clone())
            .app_data(error::json_config())
            .wrap(cors)
            .wrap(request_id::RequestId)
            .configure(configure)
            .default_service(web::to(error::not_found))
    })
    .listen(listener)?
    .run();
    Ok(server)
}
