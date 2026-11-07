//! HTML → Markdown using [`htmd`](https://docs.rs/htmd).

use std::fs;

use htmd::options::{BulletListMarker, Options};

use crate::text::read_text;
use crate::{ConvertError, Converter, Format, Progress, Request, Result};

/// Elements dropped entirely: they carry no readable content.
const SKIPPED_TAGS: &[&str] = &["head", "script", "style", "noscript", "template", "iframe"];

/// Converts HTML pages into Markdown.
///
/// The input encoding is detected automatically (BOM, `<meta charset>`,
/// UTF-8, Windows-1252). Scripts, styles and the `<head>` are removed;
/// everything else is mapped to the closest Markdown construct.
#[derive(Debug, Default, Clone, Copy)]
pub struct HtmlToMarkdownConverter;

impl Converter for HtmlToMarkdownConverter {
    fn id(&self) -> &'static str {
        "html-to-markdown"
    }

    fn inputs(&self) -> &[Format] {
        &[Format::Html]
    }

    fn outputs(&self, input: Format) -> Vec<Format> {
        match input {
            Format::Html => vec![Format::Markdown],
            _ => Vec::new(),
        }
    }

    fn convert(&self, request: &Request<'_>, progress: &dyn Progress) -> Result<()> {
        let html = read_text(request.input)?;
        progress.set(0.3);

        let markdown =
            html_to_markdown(&html).map_err(|e| ConvertError::decode(request.input, e))?;
        progress.set(0.8);

        fs::write(request.output, markdown).map_err(|e| ConvertError::io(request.output, e))
    }
}

/// Converts an HTML string to Markdown, ending with exactly one newline.
pub(crate) fn html_to_markdown(html: &str) -> std::io::Result<String> {
    // Match the style most Markdown formatters produce: `- item`, `1. item`.
    let options = Options {
        bullet_list_marker: BulletListMarker::Dash,
        ul_bullet_spacing: 1,
        ol_number_spacing: 1,
        ..Options::default()
    };
    let converter = htmd::HtmlToMarkdown::builder()
        .options(options)
        .skip_tags(SKIPPED_TAGS.to_vec())
        .build();
    let markdown = converter.convert(html)?;
    Ok(format!("{}\n", markdown.trim_end()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_common_elements() {
        let md = html_to_markdown(
            "<h1>Title</h1><p>Some <strong>bold</strong> and <a href=\"https://x.y\">link</a>.</p>\
             <ul><li>one</li><li>two</li></ul>",
        )
        .unwrap();
        assert!(md.contains("# Title"), "{md}");
        assert!(md.contains("**bold**"), "{md}");
        assert!(md.contains("[link](https://x.y)"), "{md}");
        assert!(md.contains("- one\n- two"), "{md}");
    }

    #[test]
    fn drops_scripts_styles_and_head() {
        let md = html_to_markdown(
            "<html><head><title>T</title><style>p{}</style></head>\
             <body><script>alert(1)</script><p>visible</p></body></html>",
        )
        .unwrap();
        assert_eq!(md, "visible\n");
    }
}
