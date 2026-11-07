//! HTML / Markdown → PDF or PNG, rendered by an installed browser.

use std::fs;
use std::path::Path;

use url::Url;

use super::markdown::render_page;
use crate::browser::{self, Browser, PdfSettings};
use crate::text::read_text;
use crate::{
    Availability, ConvertError, ConvertOptions, Converter, Format, Progress, Request, Result,
};

/// Renders web pages exactly as Edge/Chrome would print or display them.
///
/// - **PDF**: like "Print → Save as PDF", with the paper size, orientation,
///   margins and background setting from [`crate::WebOptions`]. Headings
///   become PDF bookmarks.
/// - **PNG**: a screenshot of the *whole* page (not just the visible part)
///   at the configured window width.
/// - **Markdown** is first turned into a styled HTML page (same as the
///   Markdown → HTML conversion), written as a hidden temporary file next to
///   the source so relative images still load.
///
/// Each conversion uses its own short-lived browser with a fresh profile, so
/// no cookies or state leak between files and no browser process outlives
/// the conversion.
#[derive(Debug, Default, Clone, Copy)]
pub struct WebConverter;

impl Converter for WebConverter {
    fn id(&self) -> &'static str {
        "browser"
    }

    fn inputs(&self) -> &[Format] {
        &[Format::Html, Format::Markdown]
    }

    fn outputs(&self, input: Format) -> Vec<Format> {
        match input {
            Format::Html | Format::Markdown => vec![Format::Pdf, Format::Png],
            _ => Vec::new(),
        }
    }

    fn availability(&self) -> Availability {
        browser::availability()
    }

    fn convert(&self, request: &Request<'_>, progress: &dyn Progress) -> Result<()> {
        let exe = browser::locate().ok_or_else(|| ConvertError::Unavailable {
            converter: "browser",
            reason: "Microsoft Edge or Google Chrome is not installed".into(),
        })?;

        // Keep the temporary page (Markdown only) alive until rendering ends.
        let page = PageSource::prepare(request)?;
        progress.set(0.1);

        let mut browser = Browser::launch(&exe)?;
        progress.set(0.3);

        let web = &request.options.web;
        let bytes = match request.target {
            Format::Pdf => browser.print_pdf(&page.url, &pdf_settings(request.options))?,
            Format::Png => browser.screenshot(&page.url, web.viewport_width.clamp(320, 4096))?,
            other => unreachable!("WebConverter does not produce {other}"),
        };
        progress.set(0.9);
        drop(browser);

        fs::write(request.output, bytes).map_err(|e| ConvertError::io(request.output, e))
    }
}

/// The page to load: the HTML file itself, or a rendered Markdown page.
struct PageSource {
    url: Url,
    /// Deleted on drop; `None` for HTML input.
    _temp: Option<tempfile::TempPath>,
}

impl PageSource {
    fn prepare(request: &Request<'_>) -> Result<Self> {
        let input =
            std::path::absolute(request.input).map_err(|e| ConvertError::io(request.input, e))?;

        if request.input_format != Format::Markdown {
            return Ok(Self {
                url: file_url(&input)?,
                _temp: None,
            });
        }

        let markdown = read_text(&input)?;
        let title = input
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let html = render_page(&markdown, &title, &request.options.html);

        let dir = input.parent().unwrap_or(Path::new("."));
        let temp = tempfile::Builder::new()
            .prefix(".convertapp-")
            .suffix(".html")
            .tempfile_in(dir)
            .map_err(|e| ConvertError::io(dir, e))?
            .into_temp_path();
        fs::write(&temp, html).map_err(|e| ConvertError::io(&temp, e))?;

        Ok(Self {
            url: file_url(&temp)?,
            _temp: Some(temp),
        })
    }
}

/// `file:///…` URL for an absolute path (handles spaces and Unicode).
fn file_url(path: &Path) -> Result<Url> {
    Url::from_file_path(path)
        .map_err(|()| ConvertError::decode(path, "the path cannot be expressed as a file:// URL"))
}

/// Converts user options into the browser's page setup.
fn pdf_settings(options: &ConvertOptions) -> PdfSettings {
    let web = &options.web;
    let (paper_width, paper_height) = web.paper.inches();
    PdfSettings {
        paper_width,
        paper_height,
        landscape: web.landscape,
        margin: f64::from(web.margin_mm.min(100)) / 25.4,
        print_background: web.print_background,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PaperSize, WebOptions};

    #[test]
    fn pdf_settings_use_inches() {
        let options = ConvertOptions {
            web: WebOptions {
                paper: PaperSize::Letter,
                margin_mm: 254,
                ..WebOptions::default()
            },
            ..ConvertOptions::default()
        };
        let settings = pdf_settings(&options);
        assert_eq!((settings.paper_width, settings.paper_height), (8.5, 11.0));
        // Margins are capped at 100 mm.
        assert!((settings.margin - 100.0 / 25.4).abs() < 1e-9);
    }

    #[test]
    fn file_urls_are_escaped() {
        let path = if cfg!(windows) {
            Path::new(r"C:\Tài liệu\a b.html")
        } else {
            Path::new("/Tài liệu/a b.html")
        };
        let url = file_url(path).unwrap();
        assert!(url.as_str().starts_with("file:///"));
        assert!(url.as_str().contains("a%20b.html"), "{url}");
    }
}
