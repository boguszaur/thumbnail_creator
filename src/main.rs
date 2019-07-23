use actix_files as afs;
use actix_web::{middleware, web, App, HttpServer};
use download::*;
use reqwest::r#async::Client;
use std::time::Duration;
use std::{env, io};
use storage::*;
use thumbnail::*;

mod download;
mod storage;
mod thumbnail;
mod thumbnail_handler;

fn main() -> io::Result<()> {
    //ToDo: move to config
    let base_dir = "/images/out";
    let listen_addr = "172.17.0.2:8080";
    let shutdown_timeout = 30;
    let max_content_length = Some(3000000);
    let check_mime_type = true;
    let max_url_in_single_req = 70;
    let http_client_timeout = 5;
    let (width, height) = (100, 100);
    let exact_size = true;
    let extension = "jpg";

    env::set_var(
        "RUST_LOG",
        "image_preview_creator::thumbnail_handler=debug,actix_web=debug",
    );
    env_logger::init();
    let sys = actix_rt::System::new("thumbnail");
    HttpServer::new(move || {
        let thumbnail = ThumbnailCreator::new(ThumbnailOptions {
            width: width,
            height: height,
            exact_size: exact_size,
        });

        let storage = ThumbnailStorage::new(base_dir, width, height, extension)
            .expect("failed to initialize storage");

        let http_client = Client::builder()
            .timeout(Duration::from_secs(http_client_timeout))
            .build()
            .expect("failed to create http client");

        let downloader = Downloader::new(
            DownloadOptions {
                max_content_length: max_content_length,
                check_mime_type: check_mime_type,
            },
            http_client,
        );

        let handler_options = thumbnail_handler::HandlerOptions {
            max_url_in_single_req: max_url_in_single_req,
        };

        App::new()
            .data(handler_options)
            .data(thumbnail)
            .data(storage)
            .data(downloader)
            .wrap(middleware::Logger::default())
            .service(web::resource("/thumbnails/{filename}").name("thumbnail_url"))
            .service(
                web::scope("/api/v1").service(web::resource("/thumbnail").route(
                    web::post().to_async(
                        thumbnail_handler::handle::<ThumbnailCreator, ThumbnailStorage, Downloader>,
                    ),
                )),
            )
            .service(afs::Files::new("/thumbnails", base_dir))
    })
    .shutdown_timeout(shutdown_timeout)
    .bind(listen_addr)?
    .start();

    println!("Starting http server: {}", listen_addr);
    sys.run()
}
