//! Pixel-level adjustments applied before encoding.

use image::imageops::FilterType;
use image::{DynamicImage, Rgb, RgbImage};

use crate::Resize;

/// Applies the user's [`Resize`] setting to a raster image (never enlarges
/// with [`Resize::Fit`]).
pub(crate) fn resize(image: DynamicImage, resize: Resize) -> DynamicImage {
    let (width, height) = resize.apply(image.width(), image.height(), false);
    if (width, height) == (image.width(), image.height()) {
        image
    } else {
        image.resize_exact(width, height, FilterType::Lanczos3)
    }
}

/// Scales the image down (never up) so both sides are at most `max` pixels,
/// keeping the aspect ratio.
pub(crate) fn fit_within(image: DynamicImage, max: u32) -> DynamicImage {
    if image.width() <= max && image.height() <= max {
        image
    } else {
        image.resize(max, max, FilterType::Lanczos3)
    }
}

/// Blends a possibly transparent image onto a solid background colour.
pub(crate) fn flatten(image: &DynamicImage, background: [u8; 3]) -> RgbImage {
    if !image.color().has_alpha() {
        return image.to_rgb8();
    }

    let rgba = image.to_rgba8();
    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let [r, g, b, a] = rgba.get_pixel(x, y).0;
        let alpha = u16::from(a);
        // Standard "over" compositing in integer maths:
        // out = src·α + bg·(1−α), with α in 0..=255. Max value 255·255 + 127
        // fits in u16.
        let blend = |src: u8, bg: u8| {
            let value = (u16::from(src) * alpha + u16::from(bg) * (255 - alpha) + 127) / 255;
            u8::try_from(value).unwrap_or(u8::MAX)
        };
        Rgb([
            blend(r, background[0]),
            blend(g, background[1]),
            blend(b, background[2]),
        ])
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn flatten_blends_onto_background() {
        let mut rgba = RgbaImage::new(3, 1);
        rgba.put_pixel(0, 0, Rgba([0, 0, 0, 0])); // fully transparent
        rgba.put_pixel(1, 0, Rgba([0, 0, 0, 255])); // opaque black
        rgba.put_pixel(2, 0, Rgba([0, 0, 0, 128])); // half transparent

        let flat = flatten(&DynamicImage::ImageRgba8(rgba), [255, 255, 255]);
        assert_eq!(flat.get_pixel(0, 0).0, [255, 255, 255]);
        assert_eq!(flat.get_pixel(1, 0).0, [0, 0, 0]);
        assert_eq!(flat.get_pixel(2, 0).0, [127, 127, 127]);
    }

    #[test]
    fn fit_within_keeps_aspect_ratio_and_never_upscales() {
        let fitted = fit_within(DynamicImage::new_rgba8(600, 300), 256);
        assert_eq!((fitted.width(), fitted.height()), (256, 128));
        assert_eq!(fit_within(DynamicImage::new_rgba8(16, 16), 256).width(), 16);
    }

    #[test]
    fn resize_applies_setting() {
        let image = DynamicImage::new_rgb8(200, 100);
        let half = resize(image.clone(), Resize::Percent(50));
        assert_eq!((half.width(), half.height()), (100, 50));
        let same = resize(image, Resize::Original);
        assert_eq!((same.width(), same.height()), (200, 100));
    }
}
