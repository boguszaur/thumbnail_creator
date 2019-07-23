use failure::Fail;
use futures::future::*;
use futures::stream::*;
use log::*;
use reqwest::r#async::Client;

pub trait DownloadService {
    fn download_image(
        &self,
        url: String,
    ) -> Box<dyn Future<Item = reqwest::r#async::Chunk, Error = DownloadError>>;
}

#[derive(Clone)]
pub struct Downloader {
    opt: DownloadOptions,
    client: Client,
}
const MIME_PREFIX: &'static str = "image/";

#[derive(Clone)]
pub struct DownloadOptions {
    pub max_content_length: Option<u64>,
    pub check_mime_type: bool,
}

/*
pub fn download_image2(
    url: String,
    client: &Client,
) -> impl Future<Item = impl AsRef<[u8]>, Error = DownloadError> {
    client
        .get(&url)
        .send()
        .map_err({
            let url_owned = url.clone();
            |err| {
                debug!("download image error: {}", err);
                DownloadError::FailedGetImage {
                    url: url_owned,
                    desc: format!("{}", err),
                }
            }
        })
        .and_then({
            |res| {
                let status = res.status();
                if status != actix_web::http::StatusCode::OK {
                    return err(DownloadError::StatusCodeNotOK {
                        url: url,
                        code: status.as_str().to_owned(),
                    });
                }
                ok(res.into_body().concat2().map_err(|err| {
                    debug!("read image payload error: {}", err);
                    DownloadError::FailedParsePayload {
                        url: url,
                        desc: format!("{}", err),
                    }
                }))
            }
        })
        .and_then(|b| b)
}*/

impl DownloadService for Downloader {
    fn download_image(
        &self,
        url: String,
    ) -> Box<dyn Future<Item = reqwest::r#async::Chunk, Error = DownloadError>> {
        Box::new(
            self.client
                .get(&url)
                .send()
                .map_err({
                    let url_owned = url.clone();
                    |err| {
                        debug!("download image error: {}", err);
                        DownloadError::FailedGetImage {
                            url: url_owned,
                            desc: format!("{}", err),
                        }
                    }
                })
                .and_then({
                    let options = self.opt.clone();
                    |res| {
                        ok(validate_response(&res, options)).and_then(|_| {
                            ok(res.into_body().concat2().map_err(|err| {
                                debug!("read image payload error: {}", err);
                                DownloadError::FailedParsePayload {
                                    url: url,
                                    desc: format!("{}", err),
                                }
                            }))
                        })
                    }
                })
                .and_then(|b| b),
        )
    }
}

fn validate_response(
    res: &reqwest::r#async::Response,
    options: DownloadOptions,
) -> Result<(), DownloadError> {
    let status = res.status();
    let url = res.url().as_str().to_owned();
    if status != actix_web::http::StatusCode::OK {
        return Err(DownloadError::StatusCodeNotOK {
            url: url,
            code: status.as_str().to_owned(),
        });
    };
    if let Some(max_content_length) = options.max_content_length {
        if let Some(actual_content_length) = res.content_length() {
            if actual_content_length > max_content_length {
                return Err(DownloadError::ContentLenghtError {
                    url: url,
                    actual_content_length: actual_content_length,
                    max_content_length: max_content_length,
                });
            }
        } else {
            // ToDo: what if content length is not set in response..
            // If payload is too big, Client should timeout.
        }
    };
    if options.check_mime_type {
        let mut invalid_content_type = true;
        let mut content_type = "";
        if let Some(header) = res.headers().get(reqwest::header::CONTENT_TYPE) {
            if let Ok(header_str) = header.to_str() {
                content_type = header_str;
                if header_str.starts_with(MIME_PREFIX) {
                    invalid_content_type = false;
                }
            }
        }
        if invalid_content_type {
            return Err(DownloadError::InvalidContentType {
                url: url,
                content_type: content_type.to_owned(),
                content_type_prefix: MIME_PREFIX.to_owned(),
            });
        }
    }
    Ok(())
}

impl Downloader {
    pub fn new(opt: DownloadOptions, client: Client) -> Self {
        Downloader {
            client: client,
            opt: opt,
        }
    }
}

#[derive(Fail, Debug)]
pub enum DownloadError {
    #[fail(display = "Failed to get image (image url: '{}') error: {}", url, desc)]
    FailedGetImage { url: String, desc: String },
    #[fail(
        display = "Get image (image url: '{}') returned status code != 200: {}",
        url, code
    )]
    StatusCodeNotOK { url: String, code: String },
    #[fail(
        display = "Failed to parse image payload (image url: '{}') error: {}",
        url, desc
    )]
    FailedParsePayload { url: String, desc: String },
    #[fail(
        display = "Response from url '{}' returned content_length {} that exceeds max allowed {}",
        url, actual_content_length, max_content_length
    )]
    ContentLenghtError {
        url: String,
        actual_content_length: u64,
        max_content_length: u64,
    },
    #[fail(
        display = "Response from url '{}' returned content_type '{}'. Expecting content type starting with '{}'",
        url, content_type, content_type_prefix
    )]
    InvalidContentType {
        url: String,
        content_type: String,
        content_type_prefix: String,
    },
}