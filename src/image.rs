use std::sync::Arc;

use iced::{widget::image::Handle, Point};
use image::{GenericImageView, ImageBuffer, Luma, Pixel, Rgba};

use crate::math::Vec2u;

pub type RgbaImage = ImageBuffer<Rgba<u8>, Vec<u8>>;
pub type GrayscaleImage = ImageBuffer<Luma<u8>, Vec<u8>>;

/// Creates a new image from provided one with requested size. It also allows to have the new image be a region of the source by "zooming" in on it.
///
/// # Parameters
/// `image`  - input image to process
/// `size`   - resulting image will be of this size
/// `offset` - 2D scalar for determining which part of the source image will be sampled. (0, 0) is top left while (1, 1) is bottom right
/// `zoom`   - any value other than 1.0 will scale up or down the source image in comparison to the output, together with `offset` this allows to zoom in on specific part of the image
///
/// # Panics
/// The function will panic if an image format with more than 4 channels per pixel is used and supplied values will try to sample outside of the image bounds
pub async fn resample_image_async<T, P>(
    image: Arc<T>,
    size: Vec2u,
    offset: Point,
    zoom: f32,
) -> ImageBuffer<P, Vec<u8>>
where
    P: Pixel<Subpixel = u8>,
    T: GenericImageView<Pixel = P> + Sync + Send + 'static,
{
    let (aspect_x, aspect_y) = {
        let aspect_calc = image.width().max(image.height()) as f32;
        let aspect_x = image.height() as f32 / aspect_calc;
        let aspect_y = image.width() as f32 / aspect_calc;
        (aspect_x, aspect_y)
    };
    // offsets are used alongside zoom function to determine which part of the image (in range of 0..1) the function should zoom onto
    let scaled_offset_x = offset.x - offset.x / zoom;
    let scaled_offset_y = offset.y - offset.y / zoom;

    let worker_size = 128;
    let workers = size.y / worker_size + if size.y % worker_size > 0 { 1 } else { 0 };

    let mut threads = Vec::with_capacity(workers as usize);
    for i in 0..workers {
        let th = tokio::spawn({
            let image = image.clone();
            async move {
                let start = worker_size * i;
                let end = (start + worker_size).min(size.y);
                let mut res: Vec<u8> = Vec::with_capacity(((end - start) * size.x) as usize);
                let empty = [0; 4];
                for y in start..end {
                    for x in 0..size.x {
                        let tx = {
                            // percent location of the pixel in range 0..1
                            let mut self_percent = x as f32 / size.x as f32 * aspect_x;
                            if zoom > 0.01 {
                                let scale = self_percent / zoom;
                                self_percent = scale + scaled_offset_x;
                            }
                            let source_scaled = image.width() as f32 * self_percent;
                            source_scaled as u32
                        };
                        let ty = {
                            // percent location of the pixel in range 0..1
                            let mut self_percent = y as f32 / size.y as f32 * aspect_y;
                            if zoom > 0.01 {
                                let scale = self_percent / zoom;
                                self_percent = scale + scaled_offset_y;
                            }
                            let source_scaled = image.height() as f32 * self_percent;
                            source_scaled as u32
                        };

                        let r = if tx < image.width() && tx > 0 && ty < image.height() && ty > 0 {
                            image.get_pixel(tx, ty)
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
    let mut pixels = Vec::with_capacity((size.x * size.y * 4) as usize);
    for th in threads {
        let mut r = th.await.unwrap();
        pixels.append(&mut r);
    }
    ImageBuffer::from_raw(size.x, size.y, pixels).unwrap()
}

/// Applies a mask to the image
pub fn mask_image(image: &mut RgbaImage, mask: &GrayscaleImage) {
    image
        .pixels_mut()
        .zip(mask.pixels())
        .filter(|(_, m)| m[0] < u8::MAX)
        .for_each(|(p, m)| p[3] = m[0].min(p[3]));
}

/// Overlays foreground on top of background respecting alpha values of the image
pub fn blend_images(background: &mut RgbaImage, foreground: &RgbaImage) {
    background
        .pixels_mut()
        .zip(foreground.pixels())
        .filter(|(_, s)| s[3] > 0)
        .for_each(|(t, s)| t.blend(s))
}

pub fn image_to_handle(image: RgbaImage) -> Handle {
    Handle::from_pixels(image.width(), image.height(), image.into_raw())
}
