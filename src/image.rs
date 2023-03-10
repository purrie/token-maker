use std::{fmt::Display, sync::Arc};

use iced::{widget::image::Handle, Point, Size};
use image::{GenericImageView, ImageBuffer, Luma, Pixel, Rgba};
use serde::{Deserialize, Serialize};

pub type RgbaImage = ImageBuffer<Rgba<u8>, Vec<u8>>;
pub type GrayscaleImage = ImageBuffer<Luma<u8>, Vec<u8>>;

/// Operation markers, they hold data and denote which operation should be performed on the image
pub enum ImageOperation {
    /// Data and instruction for the beginning of the rendering process.
    Begin {
        image: Arc<RgbaImage>,
        resolution: Size<u32>,
        offset: Point,
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
}

impl ImageOperation {
    /// Creates a starting image in rendering process
    pub async fn begin(self) -> RgbaImage {
        match self {
            ImageOperation::Begin {
                image,
                resolution,
                offset,
                size,
            } => resample_image(image, resolution, offset, size).await,
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

/// Resizes the image, clipping out the image parts or adding transparent pixels to the borders
///
/// # Parameters
/// `image`        - input image to process
/// `resolution`   - desired size of the image
/// `center_point` - 2D position which should be considered as the center of the image
/// `size`         - any value other than 1.0 will scale up or down the source image in comparison to the output, together with `offset` this allows to zoom in on specific part of the image
///
/// # Panics
/// The function will panic if an image format with more than 4 channels per pixel is used and supplied values will try to sample outside of the image bounds
///
/// Panic will also happen if supplied image  or requested resolution has width or height of 0 pixels.
pub async fn resample_image<T, P>(
    image: Arc<T>,
    resolution: Size<u32>,
    center_point: Point,
    size: f32,
) -> ImageBuffer<P, Vec<u8>>
where
    P: Pixel<Subpixel = u8>,
    T: GenericImageView<Pixel = P> + Sync + Send + 'static,
{
    let aspect = {
        let aspect_x = image.width() as f32 / resolution.width as f32 * size;
        let aspect_y = image.height() as f32 / resolution.height as f32 * size;
        aspect_x.min(aspect_y)
    };

    let half = Size {
        width: resolution.width / 2,
        height: resolution.height / 2,
    };
    let source_size = Size {
        width: image.width() as i32,
        height: image.height() as i32,
    };

    let worker_size = 128;
    let workers = resolution.height / worker_size
        + if resolution.height % worker_size > 0 {
            1
        } else {
            0
        };

    let mut threads = Vec::with_capacity(workers as usize);
    for i in 0..workers {
        let th = tokio::spawn({
            let image = image.clone();
            async move {
                let start = worker_size * i;
                let end = (start + worker_size).min(resolution.height);
                let mut res: Vec<u8> =
                    Vec::with_capacity(((end - start) * resolution.width) as usize);
                let empty = [0; 4];
                for y in start..end {
                    for x in 0..resolution.width {
                        let tx = {
                            // calculate position in range -half.width..half.width
                            let center = x as i32 - half.width as i32;
                            // calculate position of the target pixel from the image
                            let pix = center as f32 * aspect + center_point.x;
                            pix as i32
                        };
                        let ty = {
                            // calculate position in range -half.width..half.width
                            let center = y as i32 - half.height as i32;
                            // calculate position of the target pixel from the image
                            let pix = center as f32 * aspect + center_point.y;
                            pix as i32
                        };

                        let r = if tx < source_size.width
                            && tx >= 0
                            && ty < source_size.height
                            && ty >= 0
                        {
                            image.get_pixel(tx as u32, ty as u32)
                        } else {
                            *P::from_slice(&empty)
                        };
                        for p in r.channels() {
                            res.push(*p);
                        }
                    }
                }
                res
            }
        });
        threads.push(th);
    }
    let mut pixels = Vec::with_capacity(
        (resolution.width * resolution.height * P::CHANNEL_COUNT as u32) as usize,
    );
    for th in threads {
        let mut r = th.await.unwrap();
        pixels.append(&mut r);
    }
    ImageBuffer::from_raw(resolution.width, resolution.height, pixels).unwrap()
}

/// Applies a mask to the image
/// This function requires the mask to be the same size as the base image to work correctly
pub fn mask_image(mut image: RgbaImage, mask: &GrayscaleImage) -> RgbaImage {
    image
        .pixels_mut()
        .zip(mask.pixels())
        .filter(|(_, m)| m[0] < u8::MAX)
        .for_each(|(p, m)| {
            let mask = m[0];
            let source = p[3];
            if mask < source {
                // this whole color multiplication serves purpose of preventing the image from reappearing on outside edges of the frame
                let (r, g, b) = (p[0] as f32, p[1] as f32, p[2] as f32);
                let a = mask as f32 / u8::MAX as f32;
                let (r, g, b) = (r as f32 * a, g as f32 * a, b as f32 * a);
                let (r, g, b) = (r as u8, g as u8, b as u8);
                *p = [r, g, b, mask].into()
            }
        });
    image
}

/// Overlays foreground on top of background respecting alpha values of the image
/// This function requires the overlay to be the same size as the base image to work correctly
pub fn blend_images(mut image: RgbaImage, overlay: &RgbaImage) -> RgbaImage {
    image
        .pixels_mut()
        .zip(overlay.pixels())
        .for_each(|(t, s)| t.blend(s));
    image
}

/// Transforms the image into iced image handle
pub fn image_to_handle(image: RgbaImage) -> Handle {
    Handle::from_pixels(image.width(), image.height(), image.into_raw())
}

pub fn image_arc_to_handle(image: &Arc<RgbaImage>) -> Handle {
    Handle::from_pixels(
        image.width(),
        image.height(),
        image.pixels().fold(Vec::new(), |mut v, p| {
            p.0.iter().for_each(|px| v.push(*px));
            v
        }),
    )
}

/// Converts a grayscale image to iced image
#[allow(unused)]
pub fn grayscale_to_handle(mask: &GrayscaleImage) -> Handle {
    let i = RgbaImage::from_fn(mask.width(), mask.height(), |x, y| {
        let p = mask.get_pixel(x, y);
        Rgba([p[0], p[0], p[0], 255])
    });
    image_to_handle(i)
}

/// Turns hsv color into iced rgb color. Valid value ranges are 0.0..=1.0
pub fn hsv_to_color(hue: f32, saturation: f32, value: f32) -> iced::Color {
    // if there's no saturation then we have pure grayscale, which means, only value matters
    if saturation <= 0.0 {
        let v = value;
        return iced::Color::from_rgb(v, v, v);
    }

    let hue = hue * 360.0;
    let hue = hue / 60.0;
    let hue = hue % 6.0;
    let compas = hue.floor();
    let hue_deviation = hue - compas;

    let p = value * (1.0 - saturation);
    let q = value * (1.0 - saturation * hue_deviation);
    let t = value * (1.0 - saturation * (1.0 - hue_deviation));

    let (r, g, b) = match compas as u8 {
        0 => (value, t, p),
        1 => (q, value, p),
        2 => (p, value, t),
        3 => (p, q, value),
        4 => (t, p, value),
        5 => (value, p, q),
        _ => unreachable!(),
    };

    iced::Color::from_rgb(r, g, b)
}

/// Turns the color into a hue, saturation and value components in that order
pub fn color_to_hsv(color: iced::Color) -> (f32, f32, f32) {
    let min = color.r.min(color.b.min(color.g));
    let max = color.r.max(color.b.max(color.g));
    let delta = max - min;

    // min and max are similar, which means it's grayscale
    if delta == 0.0 || max == 0.0 {
        return (0.0, 0.0, max);
    }

    let value = max;
    let saturation = delta / max;
    let hue = if color.r == max {
        (color.g - color.b) / delta
    } else if color.g == max {
        2.0 + (color.b - color.r) / delta
    } else {
        4.0 + (color.r - color.g) / delta
    } * 60.0
        / 360.0;

    if hue < 0.0 {
        (1.0 + hue, saturation, value)
    } else {
        (hue, saturation, value)
    }
}
