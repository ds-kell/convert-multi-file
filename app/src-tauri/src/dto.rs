//! Data exchanged with the web frontend over Tauri IPC.
//!
//! These types mirror `app/src/types.ts`; keep both in sync. They are
//! deliberately separate from the core types so the engine does not depend
//! on serde or on UI concerns.

use std::path::PathBuf;

use convertapp_core::{Category, ConvertOptions, Format, HtmlOptions, HtmlTheme, ImageOptions};
use serde::{Deserialize, Serialize};

/// A format as shown in the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatInfo {
    /// Canonical extension, also used as the identifier sent back by the UI.
    pub id: &'static str,
    /// Display name, e.g. "JPEG".
    pub name: &'static str,
    /// `"image"` or `"document"`.
    pub category: &'static str,
}

impl From<Format> for FormatInfo {
    fn from(format: Format) -> Self {
        let category = match format.category() {
            Category::Image => "image",
            Category::Document => "document",
            #[allow(unreachable_patterns)] // `Category` is non_exhaustive-friendly
            _ => "other",
        };
        Self {
            id: format.extension(),
            name: format.display_name(),
            category,
        }
    }
}

/// One file picked or dropped by the user.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    /// Absolute path.
    pub path: PathBuf,
    /// File name for display.
    pub name: String,
    /// Detected format, `None` if unknown or unreadable.
    pub format: Option<FormatInfo>,
    /// Why the file cannot be converted, if it cannot.
    pub error: Option<String>,
}

/// Result of [`crate::commands::inspect_files`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    /// The inspected files, in the order given.
    pub files: Vec<FileInfo>,
    /// Targets every recognised file can be converted to.
    pub targets: Vec<FormatInfo>,
}

/// HTML colour theme, spelled as in the UI.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Follow the system.
    #[default]
    Auto,
    /// Always light.
    Light,
    /// Always dark.
    Dark,
    /// No stylesheet.
    None,
}

/// Conversion settings chosen in the UI.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// JPEG quality 1–100.
    pub jpeg_quality: u8,
    /// Theme for Markdown → HTML.
    pub theme: Theme,
    /// Output folder; `None` = next to each input file.
    pub output_dir: Option<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            jpeg_quality: ImageOptions::DEFAULT_JPEG_QUALITY,
            theme: Theme::Auto,
            output_dir: None,
        }
    }
}

impl Settings {
    /// Converts UI settings into engine options.
    pub fn to_options(&self) -> ConvertOptions {
        let theme = match self.theme {
            Theme::Auto => HtmlTheme::Auto,
            Theme::Light => HtmlTheme::Light,
            Theme::Dark => HtmlTheme::Dark,
            Theme::None => HtmlTheme::None,
        };
        ConvertOptions {
            image: ImageOptions {
                jpeg_quality: self.jpeg_quality,
                ..ImageOptions::default()
            },
            html: HtmlOptions { theme },
        }
    }
}

/// Progress events streamed to the UI while converting.
///
/// `index` is the position of the file in the list sent to
/// [`crate::commands::convert_files`].
#[derive(Debug, Clone, Serialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "event",
    content = "data"
)]
pub enum ConvertEvent {
    /// Work on a file started.
    Started { index: usize },
    /// Fraction (0–1) of the current file done.
    Progress { index: usize, fraction: f32 },
    /// File converted; `output` is the written file.
    Done { index: usize, output: PathBuf },
    /// File left untouched, e.g. it is already in the target format.
    Skipped { index: usize, reason: String },
    /// Conversion failed.
    Failed { index: usize, message: String },
    /// All files processed.
    Finished {
        converted: usize,
        skipped: usize,
        failed: usize,
    },
}
