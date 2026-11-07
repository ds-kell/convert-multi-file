//! Data exchanged with the web frontend over Tauri IPC.
//!
//! These types mirror `app/src/types.ts`; keep both in sync. They are
//! deliberately separate from the core types so the engine does not depend
//! on serde or on UI concerns.

use std::path::PathBuf;

use convertapp_core::{
    Category, ConvertOptions, DocumentOptions, Format, HtmlOptions, HtmlTheme, ImageOptions,
    OfficeEngine, OfficeOptions, PaperSize, PdfPageSize, Resize, StitchDirection, StitchOptions,
    WebOptions,
};
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
    pub error: Option<FileIssue>,
}

/// Why a file cannot be converted. Sent as a code (not a sentence) so the
/// UI can show it in the user's language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum FileIssue {
    /// Neither content nor extension is a known format.
    UnknownFormat,
    /// Known format, but nothing can convert it.
    Unsupported,
    /// Convertible after installing a program; `reason` says which (English,
    /// includes the install command).
    NeedsTool { reason: String },
    /// The file could not be read; `detail` is the OS error.
    Unreadable { detail: String },
}

/// Result of [`crate::commands::inspect_files`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    /// The inspected files, in the order given.
    pub files: Vec<FileInfo>,
    /// Targets every recognised file can be converted to.
    pub targets: Vec<FormatInfo>,
    /// Formats all files can be merged into, as one output (see
    /// [`convertapp_core::merge_targets`]).
    pub merge_targets: Vec<FormatInfo>,
    /// Targets that would be possible after installing a missing tool.
    pub locked: Vec<LockedInfo>,
}

/// A target that needs a missing tool, see [`convertapp_core::LockedTarget`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LockedInfo {
    /// The locked format.
    pub format: FormatInfo,
    /// Why it is locked and how to unlock it.
    pub reason: String,
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

/// Image resizing as chosen in the UI.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(
    tag = "mode",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ResizeSetting {
    /// Keep the original size.
    #[default]
    Original,
    /// Fit inside a box; a missing side is unconstrained.
    Fit {
        max_width: Option<u32>,
        max_height: Option<u32>,
    },
    /// Scale by a percentage.
    Percent { percent: u32 },
}

impl From<ResizeSetting> for Resize {
    fn from(setting: ResizeSetting) -> Self {
        match setting {
            ResizeSetting::Original => Resize::Original,
            ResizeSetting::Fit {
                max_width,
                max_height,
            } => Resize::Fit {
                max_width,
                max_height,
            },
            ResizeSetting::Percent { percent } => Resize::Percent(percent.max(1)),
        }
    }
}

/// PDF page size, spelled as in the UI.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PdfPage {
    /// A4 with margins.
    #[default]
    A4,
    /// Page as large as the image.
    Image,
}

/// Paper size for printing web pages, spelled as in the UI.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Paper {
    /// 210 × 297 mm.
    #[default]
    A4,
    /// 297 × 420 mm.
    A3,
    /// 148 × 210 mm.
    A5,
    /// US Letter.
    Letter,
    /// US Legal.
    Legal,
}

impl From<Paper> for PaperSize {
    fn from(paper: Paper) -> Self {
        match paper {
            Paper::A4 => PaperSize::A4,
            Paper::A3 => PaperSize::A3,
            Paper::A5 => PaperSize::A5,
            Paper::Letter => PaperSize::Letter,
            Paper::Legal => PaperSize::Legal,
        }
    }
}

/// Image stacking direction, spelled as in the UI.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// One below the other.
    #[default]
    Vertical,
    /// Side by side.
    Horizontal,
}

impl From<Direction> for StitchDirection {
    fn from(direction: Direction) -> Self {
        match direction {
            Direction::Vertical => StitchDirection::Vertical,
            Direction::Horizontal => StitchDirection::Horizontal,
        }
    }
}

/// How images are joined into one picture, as chosen in the UI.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StitchSetting {
    /// Stacking direction.
    pub direction: Direction,
    /// Pixels between images.
    pub gap: u32,
    /// Shrink images to a common width/height.
    pub same_size: bool,
}

impl Default for StitchSetting {
    fn default() -> Self {
        Self {
            direction: Direction::Vertical,
            gap: 0,
            same_size: true,
        }
    }
}

/// Office suite choice, spelled as in the UI.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    /// Microsoft Office if installed, else LibreOffice.
    #[default]
    Auto,
    /// Only Microsoft Office.
    Microsoft,
    /// Only LibreOffice.
    Libreoffice,
}

impl From<Engine> for OfficeEngine {
    fn from(engine: Engine) -> Self {
        match engine {
            Engine::Auto => OfficeEngine::Auto,
            Engine::Microsoft => OfficeEngine::MicrosoftOffice,
            Engine::Libreoffice => OfficeEngine::LibreOffice,
        }
    }
}

/// Page setup for HTML/Markdown → PDF/PNG, as chosen in the UI.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WebSetting {
    /// Paper size.
    pub paper: Paper,
    /// Landscape pages.
    pub landscape: bool,
    /// Margin in millimetres.
    pub margin_mm: u32,
    /// Print backgrounds.
    pub print_background: bool,
    /// Screenshot width in pixels.
    pub viewport_width: u32,
}

impl Default for WebSetting {
    fn default() -> Self {
        let defaults = WebOptions::default();
        Self {
            paper: Paper::A4,
            landscape: defaults.landscape,
            margin_mm: defaults.margin_mm,
            print_background: defaults.print_background,
            viewport_width: defaults.viewport_width,
        }
    }
}

impl From<WebSetting> for WebOptions {
    fn from(web: WebSetting) -> Self {
        Self {
            paper: web.paper.into(),
            landscape: web.landscape,
            margin_mm: web.margin_mm.min(100),
            print_background: web.print_background,
            viewport_width: web.viewport_width.clamp(320, 4096),
        }
    }
}

/// Conversion settings chosen in the UI.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Lossy quality 1–100 (JPEG, WebP, photos in PDF).
    pub quality: u8,
    /// Lossless WebP.
    pub webp_lossless: bool,
    /// Image resizing.
    pub resize: ResizeSetting,
    /// Page size for images → PDF.
    pub pdf_page: PdfPage,
    /// Join all files into one output (PDF or picture) instead of one output per file.
    pub merge: bool,
    /// Layout when joining images into one picture.
    pub stitch: StitchSetting,
    /// Theme for Markdown → HTML.
    pub theme: Theme,
    /// Word file whose styles are applied to generated .docx files.
    pub reference_docx: Option<PathBuf>,
    /// Page setup for web pages → PDF/PNG.
    pub web: WebSetting,
    /// Office suite for Word/Excel/PowerPoint files.
    pub office_engine: Engine,
    /// Output folder; `None` = next to each input file.
    pub output_dir: Option<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            quality: ImageOptions::DEFAULT_QUALITY,
            webp_lossless: false,
            resize: ResizeSetting::Original,
            pdf_page: PdfPage::A4,
            merge: false,
            stitch: StitchSetting::default(),
            theme: Theme::Auto,
            reference_docx: None,
            web: WebSetting::default(),
            office_engine: Engine::Auto,
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
        let pdf_page = match self.pdf_page {
            PdfPage::A4 => PdfPageSize::A4,
            PdfPage::Image => PdfPageSize::Image,
        };
        ConvertOptions {
            image: ImageOptions {
                quality: self.quality,
                webp_lossless: self.webp_lossless,
                resize: self.resize.into(),
                pdf_page,
                ..ImageOptions::default()
            },
            html: HtmlOptions { theme },
            document: DocumentOptions {
                reference_docx: self.reference_docx.clone(),
            },
            web: self.web.into(),
            office: OfficeOptions {
                engine: self.office_engine.into(),
            },
            stitch: StitchOptions {
                direction: self.stitch.direction.into(),
                gap: self.stitch.gap.min(10_000),
                same_size: self.stitch.same_size,
            },
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
    /// File left untouched because it already is in the target `format`
    /// (display name, e.g. "JPEG").
    Skipped { index: usize, format: String },
    /// Conversion failed.
    Failed { index: usize, message: String },
    /// All files processed.
    Finished {
        converted: usize,
        skipped: usize,
        failed: usize,
    },
}
