//! Raster image conversion: PNG, JPEG, WebP, BMP, GIF, TIFF, ICO, and
//! images → PDF.
//!
//! Split into small steps shared with the SVG converter and PDF merging:
//! - [`decode`]: read a file and apply its EXIF orientation,
//! - [`transform`]: resize / flatten / fit,
//! - [`encode`]: write the pixels in the target format,
//! - [`pdf`]: place images on PDF pages.

pub(crate) mod decode;
pub(crate) mod encode;
pub(crate) mod pdf;
pub(crate) mod transform;

use crate::{Converter, Format, Progress, Request, Result};

/// Raster formats this module can read.
pub(crate) const RASTER_INPUTS: &[Format] = &[
    Format::Png,
    Format::Jpeg,
    Format::Webp,
    Format::Bmp,
    Format::Gif,
    Format::Tiff,
    Format::Ico,
];

/// Converts between raster image formats and from images to PDF.
///
/// Format-specific behaviour:
/// - The EXIF orientation of photos is applied, so the output is upright
///   even in viewers that ignore EXIF. Metadata is not copied.
/// - **JPEG / BMP** have no (widely supported) transparency: transparent
///   pixels are blended onto [`crate::ImageOptions::background`].
/// - **ICO** is limited to 256×256: larger images are scaled down.
/// - **GIF / WebP** input: only the first frame of animations is converted.
/// - **PDF**: one page per image, see [`crate::PdfPageSize`].
#[derive(Debug, Default, Clone, Copy)]
pub struct ImageConverter;

impl Converter for ImageConverter {
    fn id(&self) -> &'static str {
        "image"
    }

    fn inputs(&self) -> &[Format] {
        RASTER_INPUTS
    }

    fn outputs(&self, input: Format) -> Vec<Format> {
        if !RASTER_INPUTS.contains(&input) {
            return Vec::new();
        }
        encode::OUTPUTS
            .iter()
            .copied()
            .filter(|&f| f != input)
            .collect()
    }

    fn convert(&self, request: &Request<'_>, progress: &dyn Progress) -> Result<()> {
        let options = &request.options.image;

        let image = decode::load(request.input)?;
        progress.set(0.4);

        let image = transform::resize(image, options.resize);
        progress.set(0.6);

        encode::write(image, request.target, options, request.output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outputs_exclude_input_and_include_pdf() {
        let outputs = ImageConverter.outputs(Format::Png);
        assert!(!outputs.contains(&Format::Png));
        assert!(outputs.contains(&Format::Webp));
        assert!(outputs.contains(&Format::Pdf));
        assert!(ImageConverter.outputs(Format::Markdown).is_empty());
    }
}
