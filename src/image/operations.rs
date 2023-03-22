use std::sync::Arc;

use iced::{Color, Point, Size};
use image::{GenericImageView, ImageBuffer, Pixel, Rgba};

use super::{GrayscaleImage, RgbaImage, convert::pixel_to_color};

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

/// Adds color as a background to the image
pub fn underlay_color(mut image: RgbaImage, color: Color) -> RgbaImage {
    let color = [
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8,
        u8::MAX,
    ];
    let color: Rgba<u8> = color.into();
    image.pixels_mut().filter(|x| x[3] < 255).for_each(|x| {
        let mut color = color.clone();
        color.blend(&x);
        *x = color;
    });
    image
}

/// Adds background to the image using `under` as the background image
///
/// # Panics
/// This function can panic if the images are not the same resolution
pub fn underlay_image(mut image: RgbaImage, under: Arc<RgbaImage>) -> RgbaImage {
    image
        .pixels_mut()
        .zip(under.pixels())
        .filter(|(i, _)| i[3] < 255)
        .for_each(|(i, u)| {
            let mut color = u.clone();
            color.blend(&i);
            *i = color;
        });
    image
}

/// Masks a specific color from the image, making matching pixels transparent
///
/// # Parameters
/// `image` - Image to be masked
/// `color` - color to mask out in the image
/// `range` - determines how fuzzy the color matching is. Value of 0 will only match exact color while higher will match similar colors too.
/// `blending` - determines the range outside of matching colors that are close to matches to turn partially transparent. Used to soften the edges around the matches. Value of 0 turns off the functionality.
pub fn mask_color(mut image: RgbaImage, color: Color, range: f32, blending: f32) -> RgbaImage {
    let range = range.min(1.0).max(0.0).powi(2);
    let soft_border = blending.min(1.0).max(0.0).powi(2);
    let soft_border_range = range + soft_border;

    image.pixels_mut().for_each(|p| {
        let c = pixel_to_color(p);

        let r = (c.r - color.r).abs().powi(2);
        let g = (c.g - color.g).abs().powi(2);
        let b = (c.b - color.b).abs().powi(2);
        let vector_length = r + g + b;

        if vector_length <= range  {
            p[3] = 0;
        } else if vector_length < soft_border_range {
            let comb = vector_length - range;
            let shade = comb / soft_border;
            p[3] = (shade * 255.0) as u8;
        }
    });

    image
}
