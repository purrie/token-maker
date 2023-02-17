use std::sync::Arc;

use iced::{widget::image::Handle, Point, Size};
use image::{GenericImageView, ImageBuffer, Luma, Pixel, Rgba};

pub type RgbaImage = ImageBuffer<Rgba<u8>, Vec<u8>>;
pub type GrayscaleImage = ImageBuffer<Luma<u8>, Vec<u8>>;

/// Resizes the image, clipping out the image parts or adding transparent pixels to the borders
///
/// # Parameters
/// `image`  - input image to process
/// `resolution`   - desired size of the image
/// `offset` - 2D offset in pixels to move the image frame
/// `size`   - any value other than 1.0 will scale up or down the source image in comparison to the output, together with `offset` this allows to zoom in on specific part of the image
///
/// # Panics
/// The function will panic if an image format with more than 4 channels per pixel is used and supplied values will try to sample outside of the image bounds
///
/// Panic will also happen if supplied image  or requested resolution has width or height of 0 pixels.
pub async fn resize_image<T, P>(
    image: Arc<T>,
    resolution: Size<u32>,
    offset: Point,
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
                            let pix = center as f32 * aspect + offset.x;
                            pix as i32
                        };
                        let ty = {
                            // calculate position in range -half.width..half.width
                            let center = y as i32 - half.height as i32;
                            // calculate position of the target pixel from the image
                            let pix = center as f32 * aspect + offset.y;
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
