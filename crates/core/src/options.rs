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

/// Settings for raster image output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageOptions {
    /// JPEG quality from 1 (smallest) to 100 (best). Values outside the range
    /// are clamped.
    pub jpeg_quality: u8,
    /// RGB colour painted behind transparent pixels when the target format has
    /// no alpha channel (JPEG, BMP).
    pub background: [u8; 3],
}

impl ImageOptions {
    /// Default JPEG quality: visually lossless for photos at a reasonable size.
    pub const DEFAULT_JPEG_QUALITY: u8 = 90;
}

impl Default for ImageOptions {
    fn default() -> Self {
        Self {
            jpeg_quality: Self::DEFAULT_JPEG_QUALITY,
            background: [255, 255, 255],
        }
    }
}
