//! Raster image conversion (PNG, JPEG, BMP, GIF, TIFF, ICO) using the pure-Rust
//! [`image`](https://docs.rs/image) crate.

use std::fs::File;
use std::io::{BufWriter, Write};

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat, ImageReader, RgbImage};

use crate::{ConvertError, Converter, Format, ImageOptions, Progress, Request, Result};

/// Formats handled by [`ImageConverter`], both as input and output.
const FORMATS: &[Format] = &[
    Format::Png,
    Format::Jpeg,
    Format::Bmp,
    Format::Gif,
    Format::Tiff,
    Format::Ico,
];

/// Largest width/height a Windows icon can have.
const ICO_MAX_SIZE: u32 = 256;

/// Converts between common raster image formats.
///
/// Format-specific behaviour:
/// - **JPEG / BMP** have no (widely supported) transparency: transparent
///   pixels are blended onto [`ImageOptions::background`].
/// - **ICO** is limited to 256×256: larger images are scaled down, keeping the
///   aspect ratio.
/// - **GIF** input: only the first frame is converted (animations are lost).
#[derive(Debug, Default, Clone, Copy)]
pub struct ImageConverter;

impl Converter for ImageConverter {
    fn id(&self) -> &'static str {
        "image"
    }

    fn inputs(&self) -> &[Format] {
        FORMATS
    }

    fn outputs(&self, input: Format) -> Vec<Format> {
        if !FORMATS.contains(&input) {
            return Vec::new();
        }
        FORMATS.iter().copied().filter(|&f| f != input).collect()
    }

    fn convert(&self, request: &Request<'_>, progress: &dyn Progress) -> Result<()> {
        let options = &request.options.image;

        let image = decode(request)?;
        progress.set(0.4);

        let image = prepare_for(request.target, image, options);
        progress.set(0.7);

        encode(&image, request, options)
    }
}

/// Reads and decodes the input file. The pixel format is sniffed from the
/// content, so a mislabelled extension does not matter.
fn decode(request: &Request<'_>) -> Result<DynamicImage> {
    ImageReader::open(request.input)
        .map_err(|e| ConvertError::io(request.input, e))?
        .with_guessed_format()
        .map_err(|e| ConvertError::io(request.input, e))?
        .decode()
        .map_err(|e| ConvertError::decode(request.input, e))
}

/// Adapts the decoded image to what the target encoder accepts.
fn prepare_for(target: Format, image: DynamicImage, options: &ImageOptions) -> DynamicImage {
    match target {
        // No usable alpha channel: flatten onto the background colour.
        Format::Jpeg | Format::Bmp => DynamicImage::ImageRgb8(flatten(&image, options.background)),
        // Size limit + the encoder expects 8-bit RGBA.
        Format::Ico => DynamicImage::ImageRgba8(fit_within(image, ICO_MAX_SIZE).to_rgba8()),
        // The GIF encoder works on 8-bit RGBA and builds its own palette.
        Format::Gif => DynamicImage::ImageRgba8(image.to_rgba8()),
        // PNG and TIFF accept every colour type `image` can decode.
        _ => image,
    }
}

/// Writes the prepared image to the request's output path.
fn encode(image: &DynamicImage, request: &Request<'_>, options: &ImageOptions) -> Result<()> {
    let output = request.output;
    let file = File::create(output).map_err(|e| ConvertError::io(output, e))?;
    let mut writer = BufWriter::new(file);

    let encoded = match request.target {
        Format::Jpeg => {
            let quality = options.jpeg_quality.clamp(1, 100);
            image.write_with_encoder(JpegEncoder::new_with_quality(&mut writer, quality))
        }
        target => image.write_to(&mut writer, image_format(target)),
    };
    encoded.map_err(|e| ConvertError::encode(output, e))?;

    writer.flush().map_err(|e| ConvertError::io(output, e))
}

/// Maps our [`Format`] to the `image` crate's equivalent.
///
/// Only called with formats from [`FORMATS`]; anything else is a logic error.
fn image_format(format: Format) -> ImageFormat {
    match format {
        Format::Png => ImageFormat::Png,
        Format::Jpeg => ImageFormat::Jpeg,
        Format::Bmp => ImageFormat::Bmp,
        Format::Gif => ImageFormat::Gif,
        Format::Tiff => ImageFormat::Tiff,
        Format::Ico => ImageFormat::Ico,
        #[allow(unreachable_patterns)] // `Format` is non_exhaustive and grows over time.
        other => unreachable!("{other} is not an image format handled by ImageConverter"),
    }
}

/// Blends a possibly transparent image onto a solid background colour.
fn flatten(image: &DynamicImage, background: [u8; 3]) -> RgbImage {
    if !image.color().has_alpha() {
        return image.to_rgb8();
    }

    let rgba = image.to_rgba8();
    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let [r, g, b, a] = rgba.get_pixel(x, y).0;
        let alpha = u16::from(a);
        // Standard "over" compositing in integer maths:
        // out = src·α + bg·(1−α), with α in 0..=255.
        let blend = |src: u8, bg: u8| {
            let value = (u16::from(src) * alpha + u16::from(bg) * (255 - alpha) + 127) / 255;
            u8::try_from(value).unwrap_or(u8::MAX)
        };
        image::Rgb([
            blend(r, background[0]),
            blend(g, background[1]),
            blend(b, background[2]),
        ])
    })
}

/// Scales the image down (never up) so both sides are at most `max` pixels,
/// keeping the aspect ratio.
fn fit_within(image: DynamicImage, max: u32) -> DynamicImage {
    if image.width() <= max && image.height() <= max {
        image
    } else {
        image.resize(max, max, FilterType::Lanczos3)
    }
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
        let big = DynamicImage::new_rgba8(600, 300);
        let fitted = fit_within(big, 256);
        assert_eq!((fitted.width(), fitted.height()), (256, 128));

        let small = DynamicImage::new_rgba8(16, 16);
        assert_eq!(fit_within(small, 256).width(), 16);
    }

    #[test]
    fn outputs_exclude_input_and_unknown_formats() {
        let outputs = ImageConverter.outputs(Format::Png);
        assert!(!outputs.contains(&Format::Png));
        assert_eq!(outputs.len(), FORMATS.len() - 1);
    }
}
