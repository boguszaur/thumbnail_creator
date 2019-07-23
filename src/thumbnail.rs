use failure::Fail;
use image::GenericImageView;
use log::*;

pub trait ThumbnailService: Send + Sync {
    fn make_thumbnail(
        &self,
        bytes: impl AsRef<[u8]>,
    ) -> Result<image::DynamicImage, ThumbnailError>;
}

#[derive(Debug, Clone)]
pub struct ThumbnailCreator {
    opt: ThumbnailOptions,
}

#[derive(Debug, Clone)]
pub struct ThumbnailOptions {
    pub width: u32,
    pub height: u32,
    pub exact_size: bool,
}

impl ThumbnailService for ThumbnailCreator {
    fn make_thumbnail(
        &self,
        bytes: impl AsRef<[u8]>,
    ) -> Result<image::DynamicImage, ThumbnailError> {
        let img = image::load_from_memory(bytes.as_ref()).map_err(|err| {
            debug!("error while parsing image: {}", err);
            ThumbnailError::InvalidImage(err)
        })?;
        // if current dimensions are less than required, image is scaled up.
        if img.width() != self.opt.width || img.height() != self.opt.height {
            return Ok(self.resize_image(img));
        }
        Ok(img)
    }
}

impl ThumbnailCreator {
    pub fn new(options: ThumbnailOptions) -> Self {
        ThumbnailCreator { opt: options }
    }

    fn resize_image(&self, img: image::DynamicImage) -> image::DynamicImage {
        let thumbnail = if self.opt.exact_size {
            img.thumbnail_exact(self.opt.width, self.opt.height)
        } else {
            img.thumbnail(self.opt.width, self.opt.height)
        };
        return thumbnail;
    }
}

#[derive(Fail, Debug)]
pub enum ThumbnailError {
    #[fail(display = "Could not parse image: {}", _0)]
    InvalidImage(image::ImageError),
}
