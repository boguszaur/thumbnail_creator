use crate::download;
use crate::storage;
use crate::thumbnail;
use actix_web::{error, http, web, HttpRequest, HttpResponse};
use failure::Fail;
use futures::future::*;
use log::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;

#[derive(Debug, Serialize, Deserialize)]
pub struct ThumbnailRequest {
    pub urls: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThumbnailResponse {
    pub success: HashMap<String, String>,
    pub failed: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct HandlerOptions {
    pub max_url_in_single_req: u64,
}

pub fn handle<
    T: thumbnail::ThumbnailService + 'static,
    S: storage::StorageService + 'static,
    D: download::DownloadService + 'static,
>(
    thumbnail: web::Data<T>,
    storage: web::Data<S>,
    downloader: web::Data<D>,
    options: web::Data<HandlerOptions>,
    req: web::Json<ThumbnailRequest>,
    http_req: HttpRequest,
) -> Box<dyn Future<Item = actix_web::web::Json<ThumbnailResponse>, Error = HandlerError>> {
    Box::new(
        result(validate_request(&req.urls, &options)).and_then(move |urls| {
            let mut all_futures = vec![];
            for k in urls {
                all_futures.push(
                    handle_one_image(
                        thumbnail.clone(),
                        storage.clone(),
                        downloader.clone(),
                        k.clone(),
                    )
                    .map(move |img_path| (k.clone(), img_path)),
                );
            }
            join_all(all_futures)
                .map_err(|_| HandlerError::EmptyError)
                .map(move |vec| {
                    let mut response = ThumbnailResponse {
                        success: HashMap::new(),
                        failed: HashMap::new(),
                    };
                    for (key, img_path) in vec {
                        match img_path {
                            Ok(path) => {
                                match http_req.url_for("thumbnail_url", &[path]) {
                                    Ok(img_url) => {
                                        response.success.insert(key, img_url.to_string());
                                    }
                                    Err(err) => {
                                        error!("error while generating url: {}", err);
                                        response
                                            .failed
                                            .insert(key, format!("Internal server error"));
                                    }
                                };
                            }
                            Err(err) => {
                                response.failed.insert(key, format!("{}", err));
                            }
                        }
                    }
                    web::Json(response)
                })
        }),
    )
}

fn handle_one_image<
    T: thumbnail::ThumbnailService + 'static,
    S: storage::StorageService + 'static,
    D: download::DownloadService + 'static,
>(
    thumbnail: web::Data<T>,
    storage: web::Data<S>,
    downloader: web::Data<D>,
    url: String,
) -> impl Future<Item = Result<String, HandlerError>, Error = ()> {
    lazy(move || {
        downloader
            .download_image(url)
            .map_err(|err| HandlerError::DownloadError(err))
            .and_then(move |bytes| {
                result(
                    storage
                        .get_image_handle(bytes.as_ref())
                        .map_err(|err| HandlerError::StorageError(err)),
                )
                .and_then(|img_handle| {
                    let ih = img_handle.clone();
                    lazy(move || {
                        if ih.exists() {
                            return ok(Ok(ih.path()));
                        }
                        return err(());
                    })
                    .or_else(|_| {
                        web::block(move || thumbnail.make_thumbnail(bytes))
                            .map_err(|err| match err {
                                error::BlockingError::Error(thumb_err) => {
                                    HandlerError::ThumbnailError(thumb_err)
                                }
                                _ => HandlerError::BlockingCancelled(
                                    "make thumbnail operation cancelled".to_owned(),
                                ),
                            })
                            .and_then(|img| {
                                web::block(move || {
                                    storage
                                        .store_image(&img_handle, img)
                                        .map(move |_| Ok(img_handle.path()))
                                })
                                .map_err(|err| match err {
                                    error::BlockingError::Error(storage_err) => {
                                        HandlerError::StorageError(storage_err)
                                    }
                                    _ => HandlerError::BlockingCancelled(
                                        "thumbnail store operation cancelled".to_owned(),
                                    ),
                                })
                            })
                    })
                })
            })
    })
    .or_else(|err| ok(Err(err)))
}

fn validate_request(
    urls: &Vec<String>,
    handler_options: &HandlerOptions,
) -> Result<HashSet<String>, HandlerError> {
    if urls.len() < 1 {
        return Err(HandlerError::EmptyURLArray);
    }
    let unique_urls: HashSet<String> = HashSet::from_iter(urls.iter().map(|url| url.to_owned()));
    if unique_urls.len() as u64 > handler_options.max_url_in_single_req {
        return Err(HandlerError::TooManyURL(
            handler_options.max_url_in_single_req,
        ));
    }
    return Ok(unique_urls);
}

#[derive(Fail, Debug)]
pub enum HandlerError {
    #[fail(display = "not reachable error")]
    EmptyError,
    #[fail(display = "Request contains empty url array")]
    EmptyURLArray,
    #[fail(display = "Request contains more than {} unique urls", _0)]
    TooManyURL(u64),
    #[fail(display = "Could not download image: {}", _0)]
    DownloadError(download::DownloadError),
    #[fail(display = "Operation cancelled: {}", _0)]
    BlockingCancelled(String),
    #[fail(display = "Thumbnail ceation error: {}", _0)]
    ThumbnailError(thumbnail::ThumbnailError),
    #[fail(display = "Storage error: {}", _0)]
    StorageError(storage::StorageError),
}

impl error::ResponseError for HandlerError {
    fn error_response(&self) -> HttpResponse {
        match self {
            HandlerError::EmptyURLArray | HandlerError::TooManyURL(_) => {
                HttpResponse::new(http::StatusCode::BAD_REQUEST)
            }
            _ => HttpResponse::new(http::StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}
