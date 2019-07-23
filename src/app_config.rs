use crate::download::*;
use crate::storage::*;
use crate::thumbnail::*;
use crate::thumbnail_handler;
use actix_web::web;
use config::{Config, ConfigError, Environment, File};
use reqwest::r#async::Client;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub listen_ip: String,
    pub listen_port: String,
    pub shutdown_timeout: u64,
    pub max_content_length: Option<u64>,
    pub check_mime_type: bool,
    pub max_urls_in_single_req: u64,
    pub http_client_timeout: u64,
    pub thumbnail_width: u32,
    pub thumbnail_height: u32,
    pub thumbnail_exact_size: bool,
    pub storage_base_dir: String,
    pub thumbnail_extension: String,
    pub log_level: String,
}

impl AppConfig {
    pub fn new() -> Result<Self, ConfigError> {
        let mut c = Config::new();
        c.merge(File::with_name("src/default_config").required(true))?;
        c.merge(Environment::with_prefix("APP"))?;
        c.try_into()
    }
}

pub fn configure_app(cfg: &mut web::ServiceConfig, app_config: &AppConfig) -> std::io::Result<()> {
    let thumbnail = ThumbnailCreator::new(ThumbnailOptions {
        width: app_config.thumbnail_width,
        height: app_config.thumbnail_width,
        exact_size: app_config.thumbnail_exact_size,
    });

    let storage = ThumbnailStorage::new(
        &app_config.storage_base_dir,
        app_config.thumbnail_width,
        app_config.thumbnail_height,
        &app_config.thumbnail_extension,
    )
    .expect("failed to initialize storage");

    let http_client = Client::builder()
        .timeout(Duration::from_secs(app_config.http_client_timeout))
        .build()
        .expect("failed to create http client");

    let downloader = Downloader::new(
        DownloadOptions {
            max_content_length: app_config.max_content_length,
            check_mime_type: app_config.check_mime_type,
        },
        http_client,
    );

    let handler_options = thumbnail_handler::HandlerOptions {
        max_url_in_single_req: app_config.max_urls_in_single_req,
    };
    cfg.data(handler_options)
        .data(thumbnail)
        .data(storage)
        .data(downloader);
    Ok(())
}
