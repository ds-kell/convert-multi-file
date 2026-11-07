//! Running conversions from start to finish: format detection, converter
//! lookup and safe output writing.

use std::fs;
use std::path::{Path, PathBuf};

use crate::converters::image::{RASTER_INPUTS, decode, pdf::PdfBuilder, transform};
use crate::converters::svg;
use crate::{
    ConvertError, ConvertOptions, Format, Job, Progress, Registry, Request, Result, output_path,
};

/// Converts one file and returns the path of the file that was written.
///
/// Steps:
/// 1. Detect the real input format ([`Format::detect`]).
/// 2. Pick a converter from the `registry`.
/// 3. Let it write into a hidden temporary file in the destination folder.
/// 4. Move the temporary file to its final, non-clashing name
///    ([`output_path`]).
///
/// Because of step 3, a failed or interrupted conversion never leaves a
/// half-written file behind.
///
/// # Errors
/// Any [`ConvertError`]; the input file is never modified.
pub fn convert(registry: &Registry, job: &Job, progress: &dyn Progress) -> Result<PathBuf> {
    let input = job.input.as_path();
    let input_format = detect(input)?;

    if input_format == job.target {
        return Err(ConvertError::Unsupported {
            from: input_format,
            to: job.target,
        });
    }

    let converter = registry.find(input_format, job.target)?;
    tracing::debug!(
        input = %input.display(),
        from = %input_format,
        to = %job.target,
        converter = converter.id(),
        "starting conversion"
    );

    progress.set(0.0);
    let dir = destination_dir(input, job.output_dir.as_deref())?;
    let final_path = write_output(input, job.target, &dir, |temp| {
        let request = Request {
            input,
            input_format,
            output: temp,
            target: job.target,
            options: &job.options,
        };
        converter.convert(&request, progress)
    })?;

    progress.set(1.0);
    tracing::info!(output = %final_path.display(), "converted");
    Ok(final_path)
}

/// Combines several images (raster or SVG) into **one** PDF, one image per
/// page, in the order given. The file is named after the first input.
///
/// `progress` goes from 0 to 1 across all inputs.
///
/// # Errors
/// [`ConvertError::NoInput`] for an empty list, [`ConvertError::Unsupported`]
/// if an input is not an image, or any error from reading the inputs.
/// Nothing is written on failure.
pub fn merge_images_to_pdf(
    inputs: &[PathBuf],
    output_dir: Option<&Path>,
    options: &ConvertOptions,
    progress: &dyn Progress,
) -> Result<PathBuf> {
    let Some(first) = inputs.first() else {
        return Err(ConvertError::NoInput);
    };

    progress.set(0.0);
    let image_options = &options.image;
    let mut pdf = PdfBuilder::new(image_options);
    for (done, input) in inputs.iter().enumerate() {
        let image = match detect(input)? {
            Format::Svg => svg::render(input, image_options.resize)?,
            format if RASTER_INPUTS.contains(&format) => {
                transform::resize(decode::load(input)?, image_options.resize)
            }
            format => {
                return Err(ConvertError::Unsupported {
                    from: format,
                    to: Format::Pdf,
                });
            }
        };
        pdf.add_image(&image)
            .map_err(|e| ConvertError::encode(input, e))?;
        progress.set((done + 1) as f32 / (inputs.len() + 1) as f32);
    }

    let dir = destination_dir(first, output_dir)?;
    let bytes = pdf.finish();
    let final_path = write_output(first, Format::Pdf, &dir, |temp| {
        fs::write(temp, &bytes).map_err(|e| ConvertError::io(temp, e))
    })?;

    progress.set(1.0);
    tracing::info!(output = %final_path.display(), pages = inputs.len(), "merged into PDF");
    Ok(final_path)
}

/// Detects the format of `input`, failing if it is unknown.
fn detect(input: &Path) -> Result<Format> {
    Format::detect(input)
        .map_err(|e| ConvertError::io(input, e))?
        .ok_or_else(|| ConvertError::UnknownFormat {
            path: input.to_path_buf(),
        })
}

/// Runs `write` against a hidden temporary file in `dir`, then moves the file
/// to a free name derived from `name_from` and `target` (see [`output_path`]).
/// If `write` fails, the temporary file is deleted.
fn write_output(
    name_from: &Path,
    target: Format,
    dir: &Path,
    write: impl FnOnce(&Path) -> Result<()>,
) -> Result<PathBuf> {
    let temp = tempfile::Builder::new()
        .prefix(".convertapp-")
        .suffix(&format!(".{}", target.extension()))
        .tempfile_in(dir)
        .map_err(|e| ConvertError::io(dir, e))?
        // Close our handle so the converter (or an external tool) can open the
        // path itself; the file is still deleted automatically on drop.
        .into_temp_path();

    write(&temp)?;

    let final_path = output_path(name_from, target, Some(dir));
    temp.persist_noclobber(&final_path)
        .map_err(|e| ConvertError::io(&final_path, e.error))?;
    Ok(final_path)
}

/// Resolves (and creates if needed) the folder the output goes into:
/// `output_dir`, or the input's own folder.
fn destination_dir(input: &Path, output_dir: Option<&Path>) -> Result<PathBuf> {
    match output_dir {
        Some(dir) => {
            fs::create_dir_all(dir).map_err(|e| ConvertError::io(dir, e))?;
            Ok(dir.to_path_buf())
        }
        None => Ok(input
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."))
            .to_path_buf()),
    }
}
