//! Running a [`Job`] from start to finish.

use std::fs;
use std::path::{Path, PathBuf};

use crate::{ConvertError, Format, Job, Progress, Registry, Request, Result, output_path};

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
    let input_format = Format::detect(input)
        .map_err(|e| ConvertError::io(input, e))?
        .ok_or_else(|| ConvertError::UnknownFormat {
            path: input.to_path_buf(),
        })?;

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

    let dir = destination_dir(job)?;
    let temp = tempfile::Builder::new()
        .prefix(".convertapp-")
        .suffix(&format!(".{}", job.target.extension()))
        .tempfile_in(&dir)
        .map_err(|e| ConvertError::io(&dir, e))?
        // Close our handle so the converter (or an external tool) can open the
        // path itself; the file is still deleted automatically on drop.
        .into_temp_path();

    let request = Request {
        input,
        input_format,
        output: &temp,
        target: job.target,
        options: &job.options,
    };
    progress.set(0.0);
    converter.convert(&request, progress)?;

    let final_path = output_path(input, job.target, Some(&dir));
    temp.persist_noclobber(&final_path)
        .map_err(|e| ConvertError::io(&final_path, e.error))?;

    progress.set(1.0);
    tracing::info!(output = %final_path.display(), "converted");
    Ok(final_path)
}

/// Resolves (and creates if needed) the folder the output goes into.
fn destination_dir(job: &Job) -> Result<PathBuf> {
    match &job.output_dir {
        Some(dir) => {
            fs::create_dir_all(dir).map_err(|e| ConvertError::io(dir, e))?;
            Ok(dir.clone())
        }
        None => Ok(job
            .input
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."))
            .to_path_buf()),
    }
}
