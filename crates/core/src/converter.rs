//! The [`Converter`] trait every conversion backend implements.

use std::fmt;
use std::path::Path;

use crate::{ConvertOptions, Format, Progress, Result};

/// Whether a converter can run on this machine right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    /// Ready to use.
    Available,
    /// Cannot run; `reason` tells the user why and ideally how to fix it
    /// (e.g. "Pandoc is not installed – run `winget install Pandoc`").
    Unavailable {
        /// Human-readable explanation.
        reason: String,
    },
}

impl Availability {
    /// `true` for [`Availability::Available`].
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }
}

/// Everything a converter needs for one file, prepared by
/// [`crate::convert`].
///
/// The pipeline has already detected `input_format` and chosen `output`
/// (a temporary file in the destination folder), so converters only have to
/// read `input` and write `output`.
#[derive(Debug, Clone, Copy)]
pub struct Request<'a> {
    /// File to read.
    pub input: &'a Path,
    /// Detected format of `input`.
    pub input_format: Format,
    /// File to write. It may already exist (empty) and must be overwritten.
    pub output: &'a Path,
    /// Format to write.
    pub target: Format,
    /// User settings.
    pub options: &'a ConvertOptions,
}

/// A conversion backend handling a family of formats.
///
/// Implementations must be cheap to construct and stateless (or internally
/// synchronised): one instance is shared across threads by the
/// [`crate::Registry`].
pub trait Converter: Send + Sync + fmt::Debug {
    /// Stable identifier, used in logs and error messages (e.g. `"image"`).
    fn id(&self) -> &'static str;

    /// Formats this converter can read.
    fn inputs(&self) -> &[Format];

    /// Formats this converter can produce from `input`.
    /// Returns an empty list if `input` is not one of [`Self::inputs`].
    fn outputs(&self, input: Format) -> Vec<Format>;

    /// Checks whether required external tools are present.
    /// Pure-Rust converters keep the default.
    fn availability(&self) -> Availability {
        Availability::Available
    }

    /// Whether the user's settings allow this converter, e.g. "use only
    /// LibreOffice" disables the Microsoft Office converters. Disabled
    /// converters are skipped when choosing who converts a file.
    fn is_enabled(&self, _options: &ConvertOptions) -> bool {
        true
    }

    /// Performs the conversion described by `request`.
    ///
    /// Implementations should call `progress.set` with increasing values when
    /// the work takes noticeable time; the pipeline reports `1.0` at the end.
    fn convert(&self, request: &Request<'_>, progress: &dyn Progress) -> Result<()>;

    /// `true` if this converter turns `from` into `to`.
    fn supports(&self, from: Format, to: Format) -> bool {
        self.inputs().contains(&from) && self.outputs(from).contains(&to)
    }
}
