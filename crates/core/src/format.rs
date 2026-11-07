//! File formats known to ConvertApp and how to recognise them.

use std::fmt;
use std::io;
use std::path::Path;
use std::str::FromStr;

/// Broad family a [`Format`] belongs to. UIs use it to group target formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    /// Raster images (PNG, JPEG, …).
    Image,
    /// Text documents (Markdown, HTML, …).
    Document,
}

/// A file format ConvertApp can read and/or write.
///
/// The enum is `#[non_exhaustive]` because new formats are added phase by
/// phase; downstream `match`es must keep a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Format {
    /// Portable Network Graphics.
    Png,
    /// JPEG / JPG.
    Jpeg,
    /// WebP (lossy or lossless; only the first frame of animations is read).
    Webp,
    /// Windows bitmap.
    Bmp,
    /// Graphics Interchange Format (only the first frame is converted).
    Gif,
    /// Tagged Image File Format.
    Tiff,
    /// Windows icon (max. 256×256).
    Ico,
    /// Scalable Vector Graphics (input only: rendered to pixels).
    Svg,
    /// Portable Document Format (output only for now: images → PDF).
    Pdf,
    /// Markdown (CommonMark + GitHub extensions).
    Markdown,
    /// HTML web page.
    Html,
}

impl Format {
    /// Every known format, in display order. The order of the variants also
    /// defines the order targets are listed in menus.
    pub const ALL: &'static [Format] = &[
        Format::Png,
        Format::Jpeg,
        Format::Webp,
        Format::Bmp,
        Format::Gif,
        Format::Tiff,
        Format::Ico,
        Format::Svg,
        Format::Pdf,
        Format::Markdown,
        Format::Html,
    ];

    /// All file extensions accepted for this format, lowercase and without a
    /// leading dot. The first entry is the canonical one used when writing.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Format::Png => &["png"],
            Format::Jpeg => &["jpg", "jpeg", "jpe", "jfif"],
            Format::Bmp => &["bmp", "dib"],
            Format::Gif => &["gif"],
            Format::Tiff => &["tiff", "tif"],
            Format::Ico => &["ico"],
            Format::Webp => &["webp"],
            Format::Svg => &["svg", "svgz"],
            Format::Pdf => &["pdf"],
            Format::Markdown => &["md", "markdown", "mdown", "mkd"],
            Format::Html => &["html", "htm", "xhtml"],
        }
    }

    /// Canonical extension used for output files, e.g. `"jpg"`.
    pub fn extension(self) -> &'static str {
        self.extensions()[0]
    }

    /// Short human-readable name, e.g. `"JPEG"`.
    pub fn display_name(self) -> &'static str {
        match self {
            Format::Png => "PNG",
            Format::Jpeg => "JPEG",
            Format::Bmp => "BMP",
            Format::Gif => "GIF",
            Format::Tiff => "TIFF",
            Format::Ico => "ICO",
            Format::Webp => "WebP",
            Format::Svg => "SVG",
            Format::Pdf => "PDF",
            Format::Markdown => "Markdown",
            Format::Html => "HTML",
        }
    }

    /// The family this format belongs to.
    pub fn category(self) -> Category {
        match self {
            Format::Png
            | Format::Jpeg
            | Format::Webp
            | Format::Bmp
            | Format::Gif
            | Format::Tiff
            | Format::Ico
            | Format::Svg => Category::Image,
            Format::Pdf | Format::Markdown | Format::Html => Category::Document,
        }
    }

    /// `true` for text-based formats. Their content has no reliable "magic
    /// bytes" (a README often starts with `<p align="center">`, which looks
    /// like HTML; an SVG is XML), so detection trusts their extension instead.
    pub fn is_text(self) -> bool {
        matches!(self, Format::Markdown | Format::Html | Format::Svg)
    }

    /// Looks up a format by file extension. Case-insensitive; a leading dot is
    /// allowed (`".PNG"` and `"png"` both work).
    pub fn from_extension(ext: &str) -> Option<Self> {
        let ext = ext.trim_start_matches('.').to_ascii_lowercase();
        Self::ALL
            .iter()
            .copied()
            .find(|format| format.extensions().contains(&ext.as_str()))
    }

    /// Guesses the format from the path's extension only (no file access).
    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(Self::from_extension)
    }

    /// Detects the real format of an existing file.
    ///
    /// For binary formats the file's leading "magic bytes" win over its
    /// extension, so a JPEG that was renamed to `.png` is still treated as
    /// JPEG. Text formats (see [`Format::is_text`]) trust their extension.
    ///
    /// Returns `Ok(None)` when neither method recognises the file.
    ///
    /// # Errors
    /// Fails if the file cannot be opened or read.
    pub fn detect(path: &Path) -> io::Result<Option<Self>> {
        let by_extension = Self::from_path(path);
        // Always read the header: this also reports missing/unreadable files.
        let by_content =
            infer::get_from_path(path)?.and_then(|kind| Self::from_extension(kind.extension()));

        if let Some(ext) = by_extension.filter(|f| f.is_text()) {
            return Ok(Some(ext));
        }

        if let (Some(content), Some(ext)) = (by_content, by_extension)
            && content != ext
        {
            tracing::debug!(
                path = %path.display(),
                extension = %ext,
                content = %content,
                "file extension does not match its content; trusting the content"
            );
        }
        Ok(by_content.or(by_extension))
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.display_name())
    }
}

/// Error returned when parsing an unknown format name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown format '{0}'")]
pub struct ParseFormatError(String);

impl FromStr for Format {
    type Err = ParseFormatError;

    /// Parses a format from an extension or display name (`"jpg"`, `"JPEG"`).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_extension(s)
            .or_else(|| {
                Self::ALL
                    .iter()
                    .copied()
                    .find(|format| format.display_name().eq_ignore_ascii_case(s))
            })
            .ok_or_else(|| ParseFormatError(s.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_lookup_is_case_insensitive_and_accepts_dot() {
        assert_eq!(Format::from_extension("PNG"), Some(Format::Png));
        assert_eq!(Format::from_extension(".jpeg"), Some(Format::Jpeg));
        assert_eq!(Format::from_extension("tif"), Some(Format::Tiff));
        assert_eq!(Format::from_extension("exe"), None);
    }

    #[test]
    fn canonical_extension_is_first_alias() {
        assert_eq!(Format::Jpeg.extension(), "jpg");
        assert_eq!(Format::Tiff.extension(), "tiff");
    }

    #[test]
    fn every_format_round_trips_through_its_extensions() {
        for &format in Format::ALL {
            for ext in format.extensions() {
                assert_eq!(Format::from_extension(ext), Some(format), "extension {ext}");
            }
        }
    }

    #[test]
    fn parses_display_names_and_extensions() {
        assert_eq!("jpeg".parse::<Format>(), Ok(Format::Jpeg));
        assert_eq!("JPG".parse::<Format>(), Ok(Format::Jpeg));
        assert_eq!("ico".parse::<Format>(), Ok(Format::Ico));
        assert!("nope".parse::<Format>().is_err());
    }

    #[test]
    fn from_path_uses_extension() {
        assert_eq!(
            Format::from_path(Path::new("dir/photo.JPG")),
            Some(Format::Jpeg)
        );
        assert_eq!(Format::from_path(Path::new("no_extension")), None);
    }
}
