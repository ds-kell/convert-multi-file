//! The [`Converter`] trait every conversion backend implements.

use std::fmt;
use std::path::Path;

use crate::{ConvertOptions, Format, Progress, Result};

/// A program the user has to install for some conversions, with what the UI
/// needs to help them do it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Requirement {
    /// Product name, e.g. "Pandoc".
    pub name: &'static str,
    /// Official download page, opened in the browser.
    pub url: &'static str,
    /// One-line install command for the current operating system, if there
    /// is one (e.g. `winget install …`, `brew install …`, `sudo apt install …`).
    pub install_command: Option<&'static str>,
    /// `false` for paid software (Microsoft Office), so the UI can point at
    /// a free alternative first.
    pub free: bool,
}

/// Whether a converter can run on this machine right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    /// Ready to use.
    Available,
    /// Cannot run until the `missing` programs are installed.
    Unavailable {
        /// Human-readable explanation (English), for logs and the CLI.
        reason: String,
        /// Programs that must *all* be installed for this converter to work.
        /// Empty when the problem is not a missing program.
        missing: Vec<Requirement>,
    },
}

impl Availability {
    /// `true` for [`Availability::Available`].
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }

    /// Unavailable because `missing` programs are not installed; the reason
    /// lists them with how to get them.
    pub fn missing(missing: Vec<Requirement>) -> Self {
        let reason = missing
            .iter()
            .map(|req| match req.install_command {
                Some(command) => format!("{} is not installed (install: {command})", req.name),
                None => format!("{} is not installed ({})", req.name, req.url),
            })
            .collect::<Vec<_>>()
            .join("; ");
        Self::Unavailable { reason, missing }
    }

    /// Combines the availability of several independent parts: available
    /// only if all are; otherwise lists everything that is missing.
    pub fn all(parts: impl IntoIterator<Item = Availability>) -> Self {
        let missing: Vec<Requirement> = parts
            .into_iter()
            .flat_map(|part| match part {
                Availability::Available => Vec::new(),
                Availability::Unavailable { missing, .. } => missing,
            })
            .collect();
        if missing.is_empty() {
            Availability::Available
        } else {
            Availability::missing(missing)
        }
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
