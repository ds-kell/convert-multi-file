//! Joining images into one picture.

use image::imageops::{self, FilterType};
use image::{DynamicImage, RgbaImage};

use crate::{StitchDirection, StitchOptions};

/// Largest picture we build: 256 megapixels (1 GiB of RGBA in memory).
const MAX_PIXELS: u64 = 256 * 1024 * 1024;

/// Stacks `images` in `options.direction`, centred across the other axis,
/// with `options.gap` pixels between them. Uncovered areas stay transparent
/// (formats without alpha later fill them with the background colour).
pub(super) fn stitch(
    images: Vec<DynamicImage>,
    options: &StitchOptions,
) -> Result<DynamicImage, String> {
    let vertical = options.direction == StitchDirection::Vertical;
    let images = if options.same_size {
        equalise(images, vertical)
    } else {
        images
    };

    // Size along the stacking axis ("length") and across it ("breadth").
    let length = |img: &DynamicImage| if vertical { img.height() } else { img.width() };
    let breadth = |img: &DynamicImage| if vertical { img.width() } else { img.height() };

    let gaps = u64::from(options.gap) * images.len().saturating_sub(1) as u64;
    let total_length = images.iter().map(|i| u64::from(length(i))).sum::<u64>() + gaps;
    let max_breadth = images.iter().map(breadth).max().unwrap_or(0);

    if total_length * u64::from(max_breadth) > MAX_PIXELS || total_length > u64::from(u32::MAX) {
        return Err(format!(
            "the joined picture would be too large ({total_length} × {max_breadth} px)"
        ));
    }
    let total_length = total_length as u32;
    let (width, height) = if vertical {
        (max_breadth, total_length)
    } else {
        (total_length, max_breadth)
    };

    let mut canvas = RgbaImage::new(width, height); // fully transparent
    let mut offset: u32 = 0;
    for image in &images {
        let centred = (max_breadth - breadth(image)) / 2;
        let (x, y) = if vertical {
            (centred, offset)
        } else {
            (offset, centred)
        };
        imageops::overlay(&mut canvas, &image.to_rgba8(), i64::from(x), i64::from(y));
        offset += length(image) + options.gap;
    }
    Ok(DynamicImage::ImageRgba8(canvas))
}

/// Shrinks images to the smallest breadth (width when stacking vertically,
/// height when side by side), keeping their aspect ratio. Never enlarges.
fn equalise(images: Vec<DynamicImage>, vertical: bool) -> Vec<DynamicImage> {
    let breadth = |img: &DynamicImage| if vertical { img.width() } else { img.height() };
    let Some(target) = images.iter().map(breadth).min() else {
        return images;
    };

    images
        .into_iter()
        .map(|image| {
            let current = breadth(&image);
            if current == target {
                return image;
            }
            let scale = f64::from(target) / f64::from(current);
            let (w, h) = if vertical {
                (
                    target,
                    ((f64::from(image.height()) * scale).round() as u32).max(1),
                )
            } else {
                (
                    ((f64::from(image.width()) * scale).round() as u32).max(1),
                    target,
                )
            };
            image.resize_exact(w, h, FilterType::Lanczos3)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(w, h, image::Rgba([255, 0, 0, 255])))
    }

    #[test]
    fn vertical_with_same_size_shrinks_to_narrowest() {
        let joined = stitch(
            vec![solid(100, 50), solid(200, 200)],
            &StitchOptions::default(),
        )
        .unwrap();
        // 200×200 shrunk to 100×100; total 100 × (50 + 100).
        assert_eq!((joined.width(), joined.height()), (100, 150));
    }

    #[test]
    fn horizontal_with_gap_and_original_sizes() {
        let options = StitchOptions {
            direction: StitchDirection::Horizontal,
            gap: 10,
            same_size: false,
        };
        let joined = stitch(vec![solid(30, 20), solid(40, 60)], &options).unwrap();
        assert_eq!((joined.width(), joined.height()), (80, 60));
        // The shorter image is centred vertically: rows above it stay clear.
        assert_eq!(joined.to_rgba8().get_pixel(5, 5).0[3], 0);
        assert_eq!(joined.to_rgba8().get_pixel(5, 30).0, [255, 0, 0, 255]);
        // The gap is transparent.
        assert_eq!(joined.to_rgba8().get_pixel(35, 30).0[3], 0);
    }

    #[test]
    fn refuses_absurd_sizes() {
        let options = StitchOptions {
            gap: u32::MAX / 2,
            ..StitchOptions::default()
        };
        assert!(stitch(vec![solid(10, 10), solid(10, 10), solid(10, 10)], &options).is_err());
    }
}
