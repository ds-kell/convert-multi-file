//! # convertapp-core
//!
//! The conversion engine behind ConvertApp. It is UI-agnostic: the CLI, the
//! desktop app and the Explorer context menu all go through this crate.
//!
//! ## Building blocks
//!
//! - [`Format`] – every file format the app knows, plus detection helpers.
//! - [`Converter`] – one implementation per conversion family
//!   (images, markdown, …). Each declares which inputs/outputs it handles.
//! - [`Registry`] – the set of available converters; answers
//!   "what can this file be converted to?".
//! - [`convert`] – runs a [`Job`] end to end: detect the input format, pick a
//!   converter, write to a temporary file and move it into place.
//!
//! ## Example
//!
//! ```no_run
//! use convertapp_core::{convert, Format, Job, NoProgress, Registry};
//!
//! let registry = Registry::with_defaults();
//! let job = Job::new("photo.png", Format::Jpeg);
//! let written = convert(&registry, &job, &NoProgress)?;
//! println!("saved to {}", written.display());
//! # Ok::<(), convertapp_core::ConvertError>(())
//! ```

#![warn(missing_docs)]

pub mod browser;
mod converter;
pub mod converters;
mod error;
mod format;
mod job;
mod options;
mod output;
mod pipeline;
mod progress;
mod registry;
pub mod text;
pub mod tools;

pub use converter::{Availability, Converter, Request};
pub use error::{BoxError, ConvertError, Result, error_chain};
pub use format::{Category, Format, ParseFormatError};
pub use job::Job;
pub use options::{
    ConvertOptions, DocumentOptions, HtmlOptions, HtmlTheme, ImageOptions, PaperSize, PdfPageSize,
    Resize, WebOptions,
};
pub use output::output_path;
pub use pipeline::{convert, merge_images_to_pdf};
pub use progress::{NoProgress, Progress};
pub use registry::{LockedTarget, Registry};
