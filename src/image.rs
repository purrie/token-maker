use iced::{widget::image::Handle, Point};
use image::{GenericImageView, ImageBuffer, Pixel, Rgba};

use crate::math::Vec2u;

pub type RgbaImage = ImageBuffer<Rgba<u8>, Vec<u8>>;

/// The function prepares the image for rendering process
///
/// # Parameters
/// `image` - input image to process
/// `size` - resulting image will be of this size
/// `offset` - {x, y} vector in range of 0.0 ..=1.0 with offset value, the source image will be moved by provided values as percentages
/// `zoom` - any value other than 1.0 will scale up or down the source image in comparison to the output, together with zoom this allows to zoom in on specific part of the image
pub fn resample_image<T: GenericImageView<Pixel = Rgba<u8>>>(
    image: &T,
    size: &Vec2u,
    offset: &Point,
    zoom: f32,
) -> RgbaImage {
    let mut resampled_image = RgbaImage::new(size.x, size.y);
    let iw = resampled_image.width();
    let ih = resampled_image.height();

    let aspect_calc = image.width().max(image.height()) as f32;
    // swapped because we want longer side cropped
    let aspect_y = image.width() as f32 / aspect_calc;
    let aspect_x = image.height() as f32 / aspect_calc;

    let scaled_offset_x = offset.x - offset.x / zoom;
    let scaled_offset_y = offset.y - offset.y / zoom;

    for x in 0..iw {
        for y in 0..ih {
            let tx = {
                // percent location of the pixel in range 0..1
                let mut self_percent = x as f32 / iw as f32 * aspect_x;
                if zoom > 0.01 {
                    let scale = self_percent / zoom;
                    self_percent = scale + scaled_offset_x;
                }
                let source_scaled = image.width() as f32 * self_percent;
                source_scaled as u32
            };
            let ty = {
                // percent location of the pixel in range 0..1
                let mut self_percent = y as f32 / ih as f32 * aspect_y;
                if zoom > 0.01 {
                    let scale = self_percent / zoom;
                    self_percent = scale + scaled_offset_y;
                }
                let source_scaled = image.height() as f32 * self_percent;
                source_scaled as u32
            };

            let r = if tx < image.width() && ty < image.height() {
                image.get_pixel(tx, ty)
            } else {
                [0, 0, 0, 0].into()
            };
            resampled_image.put_pixel(x, y, r);
        }
    }
    resampled_image
}

/// Overlays foreground on top of background respecting alpha values of the image
pub fn blend_images(background: &mut RgbaImage, foreground: &RgbaImage) {
    background
        .pixels_mut()
        .zip(foreground.pixels())
        .filter(|(_, s)| s[3] > 0)
        .for_each(|(t, s)| t.blend(s))
}

pub fn image_to_handle(image: &RgbaImage) -> Handle {
    Handle::from_pixels(
        image.width(),
        image.height(),
        image.pixels().fold(Vec::new(), |mut v, x| {
            x.0.iter().for_each(|x| v.push(*x));
            v
        }),
    )
}
