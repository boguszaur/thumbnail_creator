use crate::download;
use crate::storage;
use crate::thumbnail;
use actix_web::{error, http, web, HttpRequest, HttpResponse};
use failure::Fail;
use futures::future::*;
use log::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::iter::FromIterator;

#[derive(Deserialize)]
pub struct ThumbnailReq {
    pub urls: Vec<String>,
}

pub struct HandlerOptions {
    pub max_url_in_single_req: usize,
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
    req: web::Json<ThumbnailReq>,
    http_req: HttpRequest,
) -> Box<dyn Future<Item = actix_web::web::Json<HashMap<String, String>>, Error = HandlerError>> {
    Box::new(
        result(validate_request(&req.urls, &options)).and_then(move |mut hm| {
            let mut all_futures = vec![];
            let keys = hm.keys().map(|k| k.clone()).collect::<Vec<String>>();
            for k in keys {
                all_futures.push(
                    handle_one_image(
                        thumbnail.clone(),
                        storage.clone(),
                        downloader.clone(),
                        k.clone(),
                    )
                    .map(|img_path| (k, img_path)),
                );
            }
            join_all(all_futures).map(move |vec| {
                for (key, img_path) in vec {
                    let img_url_result = match http_req.url_for("thumbnail_url", &[img_path]) {
                        Ok(img_url) => img_url.to_string(),
                        Err(err) => {
                            error!("error while generating url: {}", err);
                            format!("Internal server error")
                        }
                    };
                    hm.insert(key, img_url_result);
                }
                web::Json(hm)
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
) -> impl Future<Item = String, Error = HandlerError> {
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
                            return ok(ih.path());
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
                                        .map(move |_| img_handle.path())
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
    .or_else(|err| ok(format!("{}", err)))
}

fn validate_request(
    urls: &Vec<String>,
    handler_options: &HandlerOptions,
) -> Result<HashMap<String, String>, HandlerError> {
    if urls.len() < 1 {
        return Err(HandlerError::EmptyURLArray);
    }
    let urls_map: HashMap<String, String> =
        HashMap::from_iter(urls.iter().map(|url| (url.to_owned(), "".to_owned())));
    if urls_map.keys().len() > handler_options.max_url_in_single_req {
        return Err(HandlerError::TooManyURL(
            handler_options.max_url_in_single_req,
        ));
    }
    return Ok(urls_map);
}

#[derive(Fail, Debug)]
pub enum HandlerError {
    #[fail(display = "Request contains empty url array")]
    EmptyURLArray,
    #[fail(display = "Request contains more than {} unique urls", _0)]
    TooManyURL(usize),
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
            //ToDo: define correct errors
            HandlerError::EmptyURLArray | HandlerError::TooManyURL(_) => {
                HttpResponse::new(http::StatusCode::BAD_REQUEST)
            }
            _ => HttpResponse::new(http::StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}
