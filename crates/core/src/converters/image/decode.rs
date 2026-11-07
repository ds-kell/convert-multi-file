//! Reading raster images from disk.

use std::path::Path;

use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageReader};

use crate::{ConvertError, Result};

/// Decodes the image at `path` and rotates/flips it according to its EXIF
/// orientation tag, so phone photos come out upright.
///
/// The pixel format is sniffed from the content, so a mislabelled extension
/// does not matter. For animated GIF/WebP only the first frame is returned.
pub(crate) fn load(path: &Path) -> Result<DynamicImage> {
    let mut decoder = ImageReader::open(path)
        .map_err(|e| ConvertError::io(path, e))?
        .with_guessed_format()
        .map_err(|e| ConvertError::io(path, e))?
        .into_decoder()
        .map_err(|e| ConvertError::decode(path, e))?;

    // A broken EXIF block must not prevent converting the pixels.
    let orientation = decoder.orientation().unwrap_or_else(|err| {
        tracing::debug!(path = %path.display(), %err, "ignoring unreadable EXIF orientation");
        Orientation::NoTransforms
    });

    let mut image =
        DynamicImage::from_decoder(decoder).map_err(|e| ConvertError::decode(path, e))?;
    image.apply_orientation(orientation);
    Ok(image)
}
