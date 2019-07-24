use failure::Fail;
use image;
use log::*;
use md5;
use std::fs;
use std::path::{Path, PathBuf};

pub trait StorageService: Send + Sync {
    fn get_image_handle(&self, bytes: impl AsRef<[u8]>) -> Result<ImageHandle, StorageError>;
    fn store_image(
        &self,
        handle: &ImageHandle,
        img: image::DynamicImage,
    ) -> Result<(), StorageError>;
    fn get_base_path(&self) -> PathBuf;
}

#[derive(Debug, Clone)]
pub struct ThumbnailStorage {
    base_path: PathBuf,
    sub_path: PathBuf,
    full_path: PathBuf,
    ext: String,
}

impl StorageService for ThumbnailStorage {
    fn get_image_handle(&self, bytes: impl AsRef<[u8]>) -> Result<ImageHandle, StorageError> {
        let img_filename = format!("{:x}.{}", md5::compute(bytes.as_ref()), &self.ext).to_owned();
        let img_full_path = self.full_path.join(&img_filename).to_owned();
        return Ok(ImageHandle {
            path: self
                .sub_path
                .join(&img_filename)
                .to_str()
                .ok_or(StorageError::InvalidPath)
                .map_err(|err| {
                    debug!("image path convert to str error: {}", err);
                    err
                })?
                .to_owned(),
            exists: img_full_path.is_file(),
        });
    }

    fn get_base_path(&self) -> PathBuf {
        self.base_path.clone()
    }

    #[cfg(not(test))]
    fn store_image(
        &self,
        handle: &ImageHandle,
        img: image::DynamicImage,
    ) -> Result<(), StorageError> {
        handle.store_image(&self.base_path, img)
    }

    #[cfg(test)]
    fn store_image(
        &self,
        handle: &ImageHandle,
        img: image::DynamicImage,
    ) -> Result<(), StorageError> {
        Ok(())
    }
}

impl ThumbnailStorage {
    pub fn new(
        base_path: impl Into<PathBuf>,
        img_width: u32,
        img_height: u32,
        img_ext: &str,
    ) -> Result<Self, StorageError> {
        let base = base_path.into();
        let sub_path = Path::new(&format!("{}x{}", img_width, img_height)).to_owned();
        let full_path = base.join(&sub_path);
        fs::create_dir_all(&full_path).map_err(|err| {
            error!("base storage folder creation error: {}", err);
            StorageError::FailedInit(err)
        })?;
        Ok(ThumbnailStorage {
            base_path: base,
            sub_path: sub_path,
            full_path: full_path,
            ext: img_ext.to_owned(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ImageHandle {
    path: String,
    exists: bool,
}

impl ImageHandle {
    fn store_image(&self, base_path: &Path, img: image::DynamicImage) -> Result<(), StorageError> {
        img.save(base_path.join(&self.path)).map_err(|err| {
            error!("image store error: {}", err);
            StorageError::FailedStore(err)
        })
    }

    pub fn exists(&self) -> bool {
        return self.exists;
    }

    pub fn path(&self) -> String {
        return self.path.clone();
    }
}

#[derive(Fail, Debug)]
pub enum StorageError {
    #[fail(display = "Invalid file path")]
    InvalidPath,
    #[fail(display = "Storage initialization error: {}", _0)]
    FailedInit(std::io::Error),
    #[fail(display = "Store image error: {}", _0)]
    FailedStore(std::io::Error),
}
