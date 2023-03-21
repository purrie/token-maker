pub mod convert;
pub mod operations;

use std::{fmt::Display, path::PathBuf, sync::Arc};

use iced::{Color, Point, Size};
use image::{ImageBuffer, Luma, Rgba};
use serde::{Deserialize, Serialize};

use self::operations::*;

pub type RgbaImage = ImageBuffer<Rgba<u8>, Vec<u8>>;
pub type GrayscaleImage = ImageBuffer<Luma<u8>, Vec<u8>>;

/// Operation markers, they hold data and denote which operation should be performed on the image
pub enum ImageOperation {
    /// Data and instruction for the beginning of the rendering process.
    Begin {
        image: Arc<RgbaImage>,
        resolution: Size<u32>,
        focus_point: Point,
        size: f32,
    },

    /// Uses the mask image to hide parts of the rendered image, dark parts of the mask hide pixels in the result
    ///
    /// This operation expects the overlay is the same resolution as the base image
    Mask { mask: Arc<GrayscaleImage> },

    /// Overlays the image on the rendered image using alpha channel blending
    ///
    /// This operation expects the overlay is the same resolution as the base image
    Blend { overlay: Arc<RgbaImage> },

    /// Adds background to the image in solid color
    BackgroundColor(Color),

    /// Adds background to the image using another image
    ///
    /// This operation expects the both images to be the same resolution
    BackgroundImage(Arc<RgbaImage>),
}

impl ImageOperation {
    /// Creates a starting image in rendering process
    pub async fn begin(self) -> RgbaImage {
        match self {
            ImageOperation::Begin {
                image,
                resolution,
                focus_point,
                size,
            } => resample_image(image, resolution, focus_point, size).await,
            _ => panic!("Image processing began on a wrong operation"),
        }
    }
    /// Performs the operation on the image, returning the result
    pub async fn perform(self, image: RgbaImage) -> RgbaImage {
        match self {
            ImageOperation::Begin { .. } => {
                panic!("Tried to call Begin operation as not a first operation!")
            }
            ImageOperation::Mask { mask } => mask_image(image, mask.as_ref()),
            ImageOperation::Blend { overlay } => blend_images(image, overlay.as_ref()),
            ImageOperation::BackgroundColor(color) => underlay_color(image, color),
            ImageOperation::BackgroundImage(under) => underlay_image(image, under),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    WebP,
    Jpeg,
    Png,
}
impl ImageFormat {
    pub const EXPORTABLE: [ImageFormat; 3] =
        [ImageFormat::WebP, ImageFormat::Jpeg, ImageFormat::Png];
}

impl Display for ImageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::WebP => "webp",
                Self::Jpeg => "jpeg",
                Self::Png => "png",
            }
        )
    }
}

pub fn image_filter(path: &PathBuf) -> bool {
    let Some(ext) = path.extension().and_then(|x| Some(x.to_string_lossy().to_lowercase())) else {
        return false;
    };

    match ext.as_str() {
        "png" | "webp" | "jpg" | "jpeg" => true,
        _ => false,
    }
}

pub async fn download_image(url: String) -> Result<RgbaImage, String> {
    let Ok(res) = reqwest::get(url).await else {
        return Err("Error: Clipboard doesn't contain a valid URL".to_string());
    };
    let Ok(btes) = res.bytes().await else {
        return Err("Error: Couldn't download image".to_string());
    };
    let Ok(img) = image::load_from_memory(&btes) else {
        return Err("Error: URL doesn't point to a valid image".to_string());
    };
    let img = img.into_rgba8();
    Ok(img)
}
