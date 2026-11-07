//! Writing decoded pixels in a target format.

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageFormat};

use super::pdf::PdfBuilder;
use super::transform::{fit_within, flatten};
use crate::{ConvertError, Format, ImageOptions, Result};

/// Every format [`write`] can produce.
pub(crate) const OUTPUTS: &[Format] = &[
    Format::Png,
    Format::Jpeg,
    Format::Webp,
    Format::Bmp,
    Format::Gif,
    Format::Tiff,
    Format::Ico,
    Format::Pdf,
];

/// Largest width/height a Windows icon can have.
const ICO_MAX_SIZE: u32 = 256;

/// Writes `image` to `output` as `target`.
///
/// Resizing must already have been applied; this only adapts the pixels to
/// what the target format supports (alpha, size limits, colour types).
pub(crate) fn write(
    image: DynamicImage,
    target: Format,
    options: &ImageOptions,
    output: &Path,
) -> Result<()> {
    match target {
        Format::Pdf => {
            let mut pdf = PdfBuilder::new(options);
            pdf.add_image(&image)
                .map_err(|e| ConvertError::encode(output, e))?;
            fs::write(output, pdf.finish()).map_err(|e| ConvertError::io(output, e))
        }
        Format::Webp => write_webp(&image, options, output),
        _ => write_with_image_crate(prepare_for(target, image, options), target, options, output),
    }
}

/// Adapts the decoded image to what the `image` crate encoder for `target`
/// accepts.
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

/// Encodes PNG, JPEG, BMP, GIF, TIFF and ICO with the `image` crate.
fn write_with_image_crate(
    image: DynamicImage,
    target: Format,
    options: &ImageOptions,
    output: &Path,
) -> Result<()> {
    let file = File::create(output).map_err(|e| ConvertError::io(output, e))?;
    let mut writer = BufWriter::new(file);

    let encoded = match target {
        Format::Jpeg => image.write_with_encoder(JpegEncoder::new_with_quality(
            &mut writer,
            options.quality.clamp(1, 100),
        )),
        _ => image.write_to(&mut writer, image_format(target)),
    };
    encoded.map_err(|e| ConvertError::encode(output, e))?;

    writer.flush().map_err(|e| ConvertError::io(output, e))
}

/// Encodes WebP with libwebp, which (unlike the `image` crate) supports lossy
/// compression. Images without transparency are encoded as RGB, which is
/// smaller.
fn write_webp(image: &DynamicImage, options: &ImageOptions, output: &Path) -> Result<()> {
    let (width, height) = (image.width(), image.height());
    let quality = f32::from(options.quality.clamp(1, 100));

    let encoded = if image.color().has_alpha() {
        let rgba = image.to_rgba8();
        webp::Encoder::from_rgba(&rgba, width, height)
            .encode_simple(options.webp_lossless, quality)
            .map(|memory| memory.to_vec())
    } else {
        let rgb = image.to_rgb8();
        webp::Encoder::from_rgb(&rgb, width, height)
            .encode_simple(options.webp_lossless, quality)
            .map(|memory| memory.to_vec())
    }
    .map_err(|e| ConvertError::encode(output, format!("WebP encoder failed: {e:?}")))?;

    fs::write(output, encoded).map_err(|e| ConvertError::io(output, e))
}

/// Maps our [`Format`] to the `image` crate's equivalent.
///
/// Only called with formats handled by [`write_with_image_crate`]; anything
/// else is a logic error.
fn image_format(format: Format) -> ImageFormat {
    match format {
        Format::Png => ImageFormat::Png,
        Format::Jpeg => ImageFormat::Jpeg,
        Format::Bmp => ImageFormat::Bmp,
        Format::Gif => ImageFormat::Gif,
        Format::Tiff => ImageFormat::Tiff,
        Format::Ico => ImageFormat::Ico,
        other => unreachable!("{other} is not written by the image crate"),
    }
}
