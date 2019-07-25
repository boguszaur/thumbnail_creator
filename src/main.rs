use actix_files as afs;
use actix_web::{middleware, web, App, HttpServer};
use app_config::*;
use download::*;
use log::*;
use std::env;
use std::io;
use storage::*;
use thumbnail::*;
use thumbnail_handler::*;
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
                    web::post().to_async(handle::<ThumbnailCreator, ThumbnailStorage, Downloader>),
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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;
    use rand;
    use std::path::Path;

    #[test]
    fn test_multiple_images() {
        call_thumbnail_handler(
            get_from_file("test_data/in/test_multiple_images/request.json"),
            Ok(get_from_file(
                "test_data/in/test_multiple_images/response.json",
            )),
            None,
        );
    }

    #[test]
    fn test_non_unique() {
        call_thumbnail_handler(
            get_from_file("test_data/in/test_non_unique/request.json"),
            Ok(get_from_file("test_data/in/test_non_unique/response.json")),
            None,
        );
    }

    #[test]
    fn test_content_length() {
        let mut config = create_config();
        config.max_content_length = Some(1000);
        call_thumbnail_handler(
            get_from_file("test_data/in/test_content_length/request.json"),
            Ok(get_from_file(
                "test_data/in/test_content_length/response.json",
            )),
            Some(config),
        );
    }

    #[test]
    fn test_content_type() {
        call_thumbnail_handler(
            get_from_file("test_data/in/test_content_type/request.json"),
            Ok(get_from_file(
                "test_data/in/test_content_type/response.json",
            )),
            None,
        );
    }

    #[test]
    fn test_incorrect_schema() {
        call_thumbnail_handler(
            get_from_file("test_data/in/test_incorrect_schema/request.json"),
            Ok(get_from_file(
                "test_data/in/test_incorrect_schema/response.json",
            )),
            None,
        );
    }

    #[test]
    fn test_invalid_url() {
        call_thumbnail_handler(
            get_from_file("test_data/in/test_invalid_url/request.json"),
            Ok(get_from_file("test_data/in/test_invalid_url/response.json")),
            None,
        );
    }

    #[test]
    fn test_no_urls() {
        call_thumbnail_handler(
            get_from_file("test_data/in/test_no_urls/request.json"),
            Err("Request contains empty url array".to_owned()),
            None,
        );
    }

    #[test]
    fn test_status_code() {
        call_thumbnail_handler(
            get_from_file("test_data/in/test_status_code/request.json"),
            Ok(get_from_file("test_data/in/test_status_code/response.json")),
            None,
        );
    }

    #[test]
    fn test_too_many_urls() {
        call_thumbnail_handler(
            get_from_file("test_data/in/test_too_many_urls/request.json"),
            Err("Request contains more than 70 unique urls".to_owned()),
            None,
        );
    }

    fn call_thumbnail_handler(
        request: ThumbnailRequest,
        expected_response: Result<ThumbnailResponse, String>,
        config: Option<AppConfig>,
    ) {
        let app_config = config.unwrap_or(create_config());
        let mut app = test::init_service(
            App::new()
                .configure(|cfg| {
                    app_config::configure_app(cfg, &app_config)
                        .expect("Error during app configuration");
                })
                .wrap(middleware::Logger::default())
                .service(web::resource("/thumbnail/{filename}").name("thumbnail_url"))
                .service(
                    web::scope("/api/v1").service(
                        web::resource("/thumbnail")
                            .route(web::post().to_async(
                                handle::<ThumbnailCreator, ThumbnailStorage, Downloader>,
                            )),
                    ),
                )
                .service(afs::Files::new("/thumbnail", &app_config.storage_base_dir)),
        );
        let req = test::TestRequest::post()
            .uri("/api/v1/thumbnail")
            .set_json(&request)
            .to_request();

        match expected_response {
            Ok(expected) => {
                let response: ThumbnailResponse = test::read_response_json(&mut app, req);
                assert_eq!(response, expected);
            }
            Err(expected) => {
                let response = test::read_response(&mut app, req)
                    .iter()
                    .map(|&u| u)
                    .collect::<Vec<u8>>();
                let text = String::from_utf8(response).unwrap();
                assert_eq!(text, expected);
            }
        };
    }

    impl std::cmp::PartialEq for ThumbnailResponse {
        fn eq(&self, other: &Self) -> bool {
            self.failed == other.failed && self.success == self.success
        }
    }

    pub fn get_from_file<T: serde::de::DeserializeOwned, P: AsRef<Path>>(path: P) -> T {
        serde_json::from_reader(std::fs::File::open(path).unwrap()).unwrap()
    }

    fn create_config() -> AppConfig {
        let out_folder = format!("test_data/out/{:x}", rand::random::<u64>());
        AppConfig {
            listen_ip: "0.0.0.0".to_owned(),
            listen_port: "8080".to_owned(),
            shutdown_timeout: 60,
            max_content_length: Some(1000000),
            check_mime_type: true,
            max_urls_in_single_req: 70,
            http_client_timeout: 5,
            thumbnail_width: 100,
            thumbnail_height: 100,
            thumbnail_exact_size: true,
            storage_base_dir: out_folder,
            thumbnail_extension: "jpg".to_owned(),
            log_level: "info".to_owned(),
        }
    }

    //delete folder after test finishes
    impl Drop for AppConfig {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.storage_base_dir);
        }
    }
}
