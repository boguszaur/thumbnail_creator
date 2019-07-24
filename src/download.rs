use bytes::Bytes;
use failure::Fail;
use futures::future::*;
use futures::stream::*;
use log::*;
use reqwest::r#async::Client;
use url::Url;

pub trait DownloadService {
    fn download_image(&self, url: String) -> Box<dyn Future<Item = Bytes, Error = DownloadError>>;
}

#[derive(Debug, Clone)]
pub struct Downloader {
    opt: DownloadOptions,
    client: Client,
}

const MIME_PREFIX: &'static str = "image/";

#[derive(Debug, Clone)]
pub struct DownloadOptions {
    pub max_content_length: Option<u64>,
    pub check_mime_type: bool,
}

impl DownloadService for Downloader {
    fn download_image(&self, url: String) -> Box<dyn Future<Item = Bytes, Error = DownloadError>> {
        let parsed_url = self.validate_url(&url);
        if parsed_url.is_err() {
            return Box::new(result(parsed_url.map(|_| Bytes::new())));
        };
        let validation_future = self
            .validate_response_header(&url, self.opt.clone())
            .map(|_| Bytes::new());
        let get_future = self.get(&url);

        Box::new(validation_future.and_then(|_| get_future))
    }
}

impl Downloader {
    pub fn new(opt: DownloadOptions, client: Client) -> Self {
        Downloader {
            client: client,
            opt: opt,
        }
    }

    fn validate_response_header(
        &self,
        url: &str,
        options: DownloadOptions,
    ) -> impl Future<Item = (), Error = DownloadError> {
        self.client
            .head(url)
            .send()
            .map_err({
                let url_owned = url.to_owned();
                |err| {
                    debug!("download image error: {}", err);
                    DownloadError::FailedGetImage {
                        url: url_owned,
                        desc: format!("{}", err),
                    }
                }
            })
            .and_then(move |res| {
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
                return Ok(());
            })
    }

    fn get(&self, url: &str) -> impl Future<Item = Bytes, Error = DownloadError> {
        self.client
            .get(url)
            .send()
            .map_err({
                let url_owned = url.to_owned();
                |err| {
                    debug!("download image error: {}", err);
                    DownloadError::FailedGetImage {
                        url: url_owned,
                        desc: format!("{}", err),
                    }
                }
            })
            .and_then({
                let url_owned = url.to_owned();
                |res| {
                    ok(res.into_body().concat2().map_err(|err| {
                        debug!("read image payload error: {}", err);
                        DownloadError::FailedParsePayload {
                            url: url_owned,
                            desc: format!("{}", err),
                        }
                    }))
                }
            })
            .and_then(|b| b)
            .map(|b| Bytes::from(b.as_ref()))
    }

    fn validate_url(&self, url: &str) -> Result<Url, DownloadError> {
        Url::parse(&url)
            .map_err(|err| DownloadError::UrsParseError {
                url: url.to_owned(),
                desc: format!("{}", err),
            })
            .and_then(|u| {
                if u.has_host() && (u.scheme() == "https" || u.scheme() == "http") {
                    return Ok(u);
                }
                return Err(DownloadError::UrsParseError {
                    url: url.to_owned(),
                    desc: "incorrect scheme or host".to_owned(),
                });
            })
    }
}

#[derive(Fail, Debug)]
pub enum DownloadError {
    #[fail(display = "Failed to parse url '{}' error: {}", url, desc)]
    UrsParseError { url: String, desc: String },
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
