//! Joining several files into **one** output:
//!
//! - images and PDFs → one PDF (each image becomes a page, each PDF keeps
//!   all its pages), in the order given;
//! - images → one picture, stacked vertically or horizontally
//!   ([`crate::StitchOptions`]).
//!
//! The result is named after the first input, like a normal conversion, and
//! is written atomically (nothing is left behind on failure).

mod pdf;
mod stitch;

use std::fs;
use std::path::{Path, PathBuf};

use image::DynamicImage;

use crate::converters::image::{RASTER_INPUTS, decode, encode, transform};
use crate::converters::svg;
use crate::pipeline::{destination_dir, detect, write_output};
use crate::{ConvertError, ConvertOptions, Format, Progress, Resize, Result};

/// Picture formats images can be joined into.
const STITCH_TARGETS: &[Format] = &[
    Format::Png,
    Format::Jpeg,
    Format::Webp,
    Format::Bmp,
    Format::Tiff,
];

/// `true` for formats [`merge`] can read as a picture (raster or SVG).
fn is_image(format: Format) -> bool {
    format == Format::Svg || RASTER_INPUTS.contains(&format)
}

/// Formats the files in `inputs` (their formats, one per file) can be merged
/// into. Empty for fewer than two files.
///
/// - Only images: PDF, or one picture (PNG, JPEG, WebP, BMP, TIFF).
/// - Images and/or PDFs: PDF.
pub fn merge_targets(inputs: &[Format]) -> Vec<Format> {
    if inputs.len() < 2 {
        return Vec::new();
    }
    if inputs.iter().all(|&f| is_image(f)) {
        let mut targets = vec![Format::Pdf];
        targets.extend_from_slice(STITCH_TARGETS);
        targets
    } else if inputs.iter().all(|&f| is_image(f) || f == Format::Pdf) {
        vec![Format::Pdf]
    } else {
        Vec::new()
    }
}

/// Joins `inputs` into one `target` file, in the order given, and returns
/// its path. `progress` goes from 0 to 1 across all inputs.
///
/// # Errors
/// [`ConvertError::NoInput`] for an empty list, [`ConvertError::Unsupported`]
/// if an input cannot be merged into `target` (see [`merge_targets`]), or
/// any error from reading the inputs. Nothing is written on failure.
pub fn merge(
    inputs: &[PathBuf],
    target: Format,
    output_dir: Option<&Path>,
    options: &ConvertOptions,
    progress: &dyn Progress,
) -> Result<PathBuf> {
    let Some(first) = inputs.first() else {
        return Err(ConvertError::NoInput);
    };

    // Validate every input before doing any work.
    let formats = inputs
        .iter()
        .map(|input| {
            let format = detect(input)?;
            let accepted = is_image(format) || (format == Format::Pdf && target == Format::Pdf);
            if accepted && (target == Format::Pdf || STITCH_TARGETS.contains(&target)) {
                Ok(format)
            } else {
                Err(ConvertError::Unsupported {
                    from: format,
                    to: target,
                })
            }
        })
        .collect::<Result<Vec<_>>>()?;

    progress.set(0.0);
    let step = |done: usize| progress.set(done as f32 / (inputs.len() + 1) as f32);

    let dir = destination_dir(first, output_dir)?;
    let final_path = if target == Format::Pdf {
        let bytes = pdf::merge_to_pdf(inputs, &formats, options, &step)?;
        write_output(first, Format::Pdf, &dir, |temp| {
            fs::write(temp, &bytes).map_err(|e| ConvertError::io(temp, e))
        })?
    } else {
        let mut images = Vec::with_capacity(inputs.len());
        for (index, (input, &format)) in inputs.iter().zip(&formats).enumerate() {
            images.push(load_image(input, format, Resize::Original)?);
            step(index + 1);
        }
        let joined = stitch::stitch(images, &options.stitch)
            .map_err(|message| ConvertError::encode(first, message))?;
        // The resize option applies to the finished picture.
        let joined = transform::resize(joined, options.image.resize);
        write_output(first, target, &dir, |temp| {
            encode::write(joined, target, &options.image, temp)
        })?
    };

    progress.set(1.0);
    tracing::info!(output = %final_path.display(), files = inputs.len(), "merged");
    Ok(final_path)
}

/// Reads one image (raster or SVG) at the size chosen by `resize`.
fn load_image(path: &Path, format: Format, resize: Resize) -> Result<DynamicImage> {
    if format == Format::Svg {
        svg::render(path, resize)
    } else {
        Ok(transform::resize(decode::load(path)?, resize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_depend_on_the_mix_of_inputs() {
        assert!(merge_targets(&[Format::Png]).is_empty(), "needs two files");

        let images = merge_targets(&[Format::Png, Format::Jpeg, Format::Svg]);
        assert!(images.contains(&Format::Pdf) && images.contains(&Format::Png));

        assert_eq!(merge_targets(&[Format::Png, Format::Pdf]), [Format::Pdf]);
        assert_eq!(merge_targets(&[Format::Pdf, Format::Pdf]), [Format::Pdf]);
        assert!(merge_targets(&[Format::Pdf, Format::Docx]).is_empty());
    }
}
