//! A single "convert this file to that format" request.

use std::path::PathBuf;

use crate::{ConvertOptions, Format};

/// One file to convert, as requested by the user.
///
/// Build it with [`Job::new`] and the `with_*` methods, then pass it to
/// [`crate::convert`].
#[derive(Debug, Clone)]
pub struct Job {
    /// The file to convert.
    pub input: PathBuf,
    /// Desired output format.
    pub target: Format,
    /// Where to save the result. `None` = next to the input file.
    pub output_dir: Option<PathBuf>,
    /// Conversion settings.
    pub options: ConvertOptions,
}

impl Job {
    /// Creates a job with default options, saving next to the input file.
    pub fn new(input: impl Into<PathBuf>, target: Format) -> Self {
        Self {
            input: input.into(),
            target,
            output_dir: None,
            options: ConvertOptions::default(),
        }
    }

    /// Saves the result into `dir` instead of next to the input.
    #[must_use]
    pub fn with_output_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.output_dir = Some(dir.into());
        self
    }

    /// Replaces the conversion settings.
    #[must_use]
    pub fn with_options(mut self, options: ConvertOptions) -> Self {
        self.options = options;
        self
    }
}
