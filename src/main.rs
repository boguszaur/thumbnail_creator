use actix_files as afs;
use actix_web::{middleware, web, App, HttpServer};
use app_config::*;
use download::*;
use log::*;
use std::env;
use std::io;
use storage::*;
use thumbnail::*;

mod app_config;
mod download;
mod storage;
mod thumbnail;
mod thumbnail_handler;

fn main() -> io::Result<()> {
    let app_config = AppConfig::new().expect("failed to create configuration");
    let listen_addr = format!("{}:{}", app_config.listen_ip, app_config.listen_port);
    let shutdown_timeout = app_config.shutdown_timeout;
    env::set_var("RUST_LOG", &app_config.log_level);
    env_logger::init();
    let sys = actix_rt::System::new("thumbnail_creator");

    HttpServer::new(move || {
        App::new()
            .configure(|cfg| {
                app_config::configure_app(cfg, &app_config)
                    .expect("Error during app configuration");
            })
            .wrap(middleware::Logger::default())
            .service(web::resource("/thumbnail/{filename}").name("thumbnail_url"))
            .service(
                web::scope("/api/v1").service(web::resource("/thumbnail").route(
                    web::post().to_async(
                        thumbnail_handler::handle::<ThumbnailCreator, ThumbnailStorage, Downloader>,
                    ),
                )),
            )
            .service(afs::Files::new("/thumbnail", &app_config.storage_base_dir))
    })
    .shutdown_timeout(shutdown_timeout)
    .bind(&listen_addr)?
    .start();

    info!("Starting http server: {}", &listen_addr);
    sys.run()
}
