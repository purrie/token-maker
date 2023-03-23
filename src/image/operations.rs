use std::sync::Arc;

use iced::{Color, Point, Size, Vector};
use image::{GenericImageView, ImageBuffer, Pixel, Primitive, Rgba};

use super::{convert::pixel_to_color, GrayscaleImage, RgbaImage};

/// Resizes the image, clipping out the image parts or adding transparent pixels to the borders
///
/// # Parameters
/// `image`        - input image to process
/// `resolution`   - desired size of the image
/// `center_point` - 2D position which should be considered as the center of the image
/// `size`         - any value other than 1.0 will scale up or down the source image in comparison to the output, together with `offset` this allows to zoom in on specific part of the image
///
/// # Panics
/// Panic will also happen if supplied image or requested resolution has width or height of 0 pixels.
pub async fn resample_image<T, P>(
    image: Arc<T>,
    resolution: Size<u32>,
    center_point: Point,
    size: f32,
) -> ImageBuffer<P, Vec<u8>>
where
    P: Pixel<Subpixel = u8> + Send + 'static,
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
            let empty = image.get_pixel(0, 0).map(|_| 0);
            async move {
                let start = worker_size * i;
                let end = (start + worker_size).min(resolution.height);
                let mut res: Vec<u8> =
                    Vec::with_capacity(((end - start) * resolution.width) as usize);
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
                            empty
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

pub async fn mask_image_with_offset(
    image: RgbaImage,
    mask: Arc<GrayscaleImage>,
    center: Point,
    size: f32,
) -> RgbaImage {
    let mask = resample_image(
        mask,
        Size {
            width: image.width(),
            height: image.height(),
        },
        center,
        size,
    )
    .await;
    mask_image(image, &mask)
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

        if vector_length <= range {
            p[3] = 0;
        } else if vector_length < soft_border_range {
            let comb = vector_length - range;
            let shade = comb / soft_border;
            p[3] = (shade * 255.0) as u8;
        }
    });

    image
}

/// Creates a grayscale image by flood filling it pixel by pixel
///
/// # Parameters
/// `image` - The image to use as basis for flooding
/// `starting_point` - the point from which start the flooding
/// `starting_value` - the value the mask will be prefilled with
/// `predicate` - function that will determine boundaries of flood and value for the mask
///     The predicate is given the pixel from the `image`, the length of the slice is equal to image channel count
///     The predicate returns value the mask will take, or None, which will stop spread from that point
pub fn flood_fill_mask<F, P, S>(
    image: &ImageBuffer<P, Vec<S>>,
    starting_point: Vector<u32>,
    starting_value: u8,
    predicate: F,
) -> GrayscaleImage
where
    F: Fn(&[S]) -> Option<u8>,
    P: Pixel<Subpixel = S>,
    S: Primitive,
{
    let size = (image.width() * image.height()) as usize;
    let (width, height) = (image.width() as usize, image.height() as usize);
    let pixels = image.as_raw();
    let mut mask = Vec::with_capacity(size);
    mask.resize(size, starting_value);
    let mut stack = Vec::new();

    // calculates linear index of a pixel
    macro_rules! index {
        ($x:expr, $y:expr) => {
            width * $y + $x
        };
    }

    // Tests the point according to predicate and assigns the value returned
    macro_rules! mark_point {
        ($x:expr, $y:expr) => {
            let i = index!($x, $y);
            if mask[i] == starting_value {
                let ci = i * P::CHANNEL_COUNT as usize;
                let cie = ci + P::CHANNEL_COUNT as usize;
                if let Some(v) = predicate(&pixels[ci..cie]) {
                    debug_assert_ne!(
                        v,
                        starting_value,
                        "Error: Flood Fill Mask predicate cannot return the same value as starting mask value, otherwise it can lead to cyclic infinite loop");
                    mask[i] = v;
                    stack.push(($x, $y));
                }
            }
        };
    }

    // performs range checks and adds pixels on each side of provided coordinate to be processed according to `add_point` rules
    macro_rules! add_around {
        ($x:expr, $y:expr) => {
            if $x > 0 {
                mark_point!($x - 1, $y);
            }
            if $x < width - 1 {
                mark_point!($x + 1, $y);
            }
            if $y > 0 {
                mark_point!($x, $y - 1);
            }
            if $y < height - 1 {
                mark_point!($x, $y + 1);
            }
        };
    }

    mark_point!(starting_point.x as usize, starting_point.y as usize);

    while let Some((x, y)) = stack.pop() {
        add_around!(x, y);
    }

    let mask = ImageBuffer::from_raw(image.width(), image.height(), mask).unwrap();
    mask
}
