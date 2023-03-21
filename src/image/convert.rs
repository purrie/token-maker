use std::sync::Arc;

use iced_native::image::Handle;
use image::Rgba;

use super::{RgbaImage, GrayscaleImage};


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
