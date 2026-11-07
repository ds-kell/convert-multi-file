//! User-tunable conversion settings.
//!
//! Options are grouped per converter family so the UI can show only the panel
//! that matters for the chosen target format.

/// All settings for a conversion. `Default` gives sensible values.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConvertOptions {
    /// Settings used by the image converter.
    pub image: ImageOptions,
    /// Settings used when producing HTML from Markdown.
    pub html: HtmlOptions,
    /// Settings for document conversions done by Pandoc.
    pub document: DocumentOptions,
    /// Settings for rendering web pages (HTML/Markdown → PDF/PNG).
    pub web: WebOptions,
    /// Settings for Word/Excel/PowerPoint documents.
    pub office: OfficeOptions,
    /// Settings for joining several images into one picture.
    pub stitch: StitchOptions,
}

/// How [`crate::merge`] lays out images joined into one picture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StitchOptions {
    /// Stack the images top-to-bottom or left-to-right.
    pub direction: StitchDirection,
    /// Empty space between images, in pixels.
    pub gap: u32,
    /// Shrink the images to a common width (vertical) or height
    /// (horizontal) so the edges line up. Images are never enlarged.
    pub same_size: bool,
}

impl Default for StitchOptions {
    fn default() -> Self {
        Self {
            direction: StitchDirection::Vertical,
            gap: 0,
            same_size: true,
        }
    }
}

/// Direction in which images are joined.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum StitchDirection {
    /// One below the other (e.g. screenshots of a long page).
    #[default]
    Vertical,
    /// Side by side.
    Horizontal,
}

/// Settings for office documents.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OfficeOptions {
    /// Which office suite converts the documents.
    pub engine: OfficeEngine,
}

/// The office suite used for Word/Excel/PowerPoint conversions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum OfficeEngine {
    /// Microsoft Office if installed (best fidelity), otherwise LibreOffice.
    #[default]
    Auto,
    /// Only Microsoft Office.
    MicrosoftOffice,
    /// Only LibreOffice.
    LibreOffice,
}

/// Page setup used when a browser renders HTML or Markdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebOptions {
    /// Paper size for PDF output.
    pub paper: PaperSize,
    /// Landscape orientation for PDF output.
    pub landscape: bool,
    /// Margin on every side of PDF pages, in millimetres.
    pub margin_mm: u32,
    /// Print background colours and images (as seen on screen).
    pub print_background: bool,
    /// Window width in CSS pixels for PNG screenshots; the height grows to
    /// fit the whole page.
    pub viewport_width: u32,
}

impl Default for WebOptions {
    fn default() -> Self {
        Self {
            paper: PaperSize::A4,
            landscape: false,
            margin_mm: 10,
            print_background: true,
            viewport_width: 1280,
        }
    }
}

/// Standard paper sizes for printing web pages.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum PaperSize {
    /// 210 × 297 mm.
    #[default]
    A4,
    /// 297 × 420 mm.
    A3,
    /// 148 × 210 mm.
    A5,
    /// US Letter, 8.5 × 11 in.
    Letter,
    /// US Legal, 8.5 × 14 in.
    Legal,
}

impl PaperSize {
    /// Portrait `(width, height)` in inches.
    pub fn inches(self) -> (f64, f64) {
        const MM: f64 = 1.0 / 25.4;
        match self {
            PaperSize::A4 => (210.0 * MM, 297.0 * MM),
            PaperSize::A3 => (297.0 * MM, 420.0 * MM),
            PaperSize::A5 => (148.0 * MM, 210.0 * MM),
            PaperSize::Letter => (8.5, 11.0),
            PaperSize::Legal => (8.5, 14.0),
        }
    }
}

/// Settings for document conversions (Word, PDF from Markdown).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocumentOptions {
    /// A `.docx` whose styles (fonts, headings, margins…) are applied to
    /// generated Word files, like Pandoc's `--reference-doc`.
    pub reference_docx: Option<std::path::PathBuf>,
}

/// Settings for HTML output (Markdown → HTML).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HtmlOptions {
    /// Colour scheme of the embedded stylesheet.
    pub theme: HtmlTheme,
}

/// Look of generated HTML pages.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum HtmlTheme {
    /// Light or dark, following the reader's system setting.
    #[default]
    Auto,
    /// Always light.
    Light,
    /// Always dark.
    Dark,
    /// No stylesheet at all: bare HTML, e.g. to paste into another page.
    None,
}

impl HtmlTheme {
    /// Value used in the `data-theme` attribute of the generated page, or
    /// `None` when no stylesheet is embedded.
    pub fn as_attribute(self) -> Option<&'static str> {
        match self {
            HtmlTheme::Auto => Some("auto"),
            HtmlTheme::Light => Some("light"),
            HtmlTheme::Dark => Some("dark"),
            HtmlTheme::None => None,
        }
    }
}

/// Settings for raster image output (and images placed into PDFs).
///
/// Metadata (EXIF, GPS, camera info…) is never copied to the output; the
/// EXIF orientation is applied to the pixels first so photos stay upright.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageOptions {
    /// Quality of lossy output (JPEG, lossy WebP, photos inside PDFs), from 1
    /// (smallest) to 100 (best). Values outside the range are clamped.
    pub quality: u8,
    /// Write WebP losslessly instead of using [`quality`](Self::quality).
    pub webp_lossless: bool,
    /// RGB colour painted behind transparent pixels when the target format has
    /// no alpha channel (JPEG, BMP).
    pub background: [u8; 3],
    /// Optional resizing applied before encoding.
    pub resize: Resize,
    /// Page layout for image → PDF.
    pub pdf_page: PdfPageSize,
}

impl ImageOptions {
    /// Default quality: visually lossless for photos at a reasonable size.
    pub const DEFAULT_QUALITY: u8 = 90;
}

impl Default for ImageOptions {
    fn default() -> Self {
        Self {
            quality: Self::DEFAULT_QUALITY,
            webp_lossless: false,
            background: [255, 255, 255],
            resize: Resize::Original,
            pdf_page: PdfPageSize::A4,
        }
    }
}

/// How to change the pixel size of an image.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Resize {
    /// Keep the original size.
    #[default]
    Original,
    /// Shrink to fit inside the box, keeping the aspect ratio. `None` leaves a
    /// side unconstrained. Raster images are never enlarged; SVGs are rendered
    /// to fit the box exactly, since vectors scale without losing quality.
    Fit {
        /// Maximum width in pixels.
        max_width: Option<u32>,
        /// Maximum height in pixels.
        max_height: Option<u32>,
    },
    /// Scale both sides by this percentage (100 = unchanged).
    Percent(u32),
}

impl Resize {
    /// Computes the new size for an image of `width`×`height`.
    ///
    /// `allow_upscale` lets [`Resize::Fit`] enlarge small images (used for
    /// SVG). The result is always at least 1×1.
    pub fn apply(self, width: u32, height: u32, allow_upscale: bool) -> (u32, u32) {
        let scale = match self {
            Resize::Original => 1.0,
            Resize::Percent(percent) => f64::from(percent) / 100.0,
            Resize::Fit {
                max_width,
                max_height,
            } => {
                let sx = max_width.map_or(f64::INFINITY, |w| f64::from(w) / f64::from(width));
                let sy = max_height.map_or(f64::INFINITY, |h| f64::from(h) / f64::from(height));
                let scale = sx.min(sy);
                if !scale.is_finite() || (!allow_upscale && scale > 1.0) {
                    1.0
                } else {
                    scale
                }
            }
        };
        let scaled = |side: u32| ((f64::from(side) * scale).round() as u32).max(1);
        (scaled(width), scaled(height))
    }
}

/// Page size used when putting images into a PDF.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum PdfPageSize {
    /// A4 with a small margin; landscape for wide images. Images are scaled
    /// down to fit, never enlarged.
    #[default]
    A4,
    /// Each page has exactly the image's size (at 96 DPI), without margins.
    Image,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_keeps_size() {
        assert_eq!(Resize::Original.apply(640, 480, false), (640, 480));
    }

    #[test]
    fn percent_scales_both_sides() {
        assert_eq!(Resize::Percent(50).apply(640, 480, false), (320, 240));
        assert_eq!(Resize::Percent(200).apply(10, 5, false), (20, 10));
    }

    #[test]
    fn fit_keeps_aspect_ratio_and_respects_upscale_flag() {
        let fit = Resize::Fit {
            max_width: Some(100),
            max_height: None,
        };
        assert_eq!(fit.apply(400, 200, false), (100, 50));
        assert_eq!(fit.apply(50, 25, false), (50, 25), "raster is not enlarged");
        assert_eq!(fit.apply(50, 25, true), (100, 50), "vector may be enlarged");

        let both = Resize::Fit {
            max_width: Some(100),
            max_height: Some(100),
        };
        assert_eq!(both.apply(400, 200, false), (100, 50));
        assert_eq!(both.apply(200, 400, false), (50, 100));
    }

    #[test]
    fn never_collapses_to_zero() {
        assert_eq!(Resize::Percent(1).apply(10, 10, false), (1, 1));
        let unconstrained = Resize::Fit {
            max_width: None,
            max_height: None,
        };
        assert_eq!(unconstrained.apply(7, 3, false), (7, 3));
    }
}
