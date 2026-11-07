//! Markdown → HTML using [`comrak`](https://docs.rs/comrak), a CommonMark +
//! GitHub Flavored Markdown parser.

use std::fs;
use std::path::Path;

use comrak::nodes::NodeValue;
use comrak::{Arena, Options, format_html, parse_document};

use crate::text::read_text;
use crate::{ConvertError, Converter, Format, HtmlOptions, Progress, Request, Result};

/// Stylesheet embedded in every generated page (unless the theme is `None`).
const STYLESHEET: &str = include_str!("../../assets/markdown.css");

/// Converts Markdown files into standalone HTML pages.
///
/// - GitHub extensions are enabled: tables, task lists, strikethrough,
///   autolinks, footnotes, alerts (`> [!NOTE]`) and heading anchors.
/// - Raw HTML inside the Markdown is kept as is. The input is the user's own
///   file, so this matches what they would see on GitHub or in an editor.
/// - The page title is taken from the first heading, or the file name.
/// - Relative links and images are not rewritten, so they keep working when
///   the HTML is saved next to the Markdown file.
#[derive(Debug, Default, Clone, Copy)]
pub struct MarkdownConverter;

impl Converter for MarkdownConverter {
    fn id(&self) -> &'static str {
        "markdown"
    }

    fn inputs(&self) -> &[Format] {
        &[Format::Markdown]
    }

    fn outputs(&self, input: Format) -> Vec<Format> {
        match input {
            Format::Markdown => vec![Format::Html],
            _ => Vec::new(),
        }
    }

    fn convert(&self, request: &Request<'_>, progress: &dyn Progress) -> Result<()> {
        let markdown = read_text(request.input)?;
        progress.set(0.3);

        let fallback_title = file_title(request.input);
        let page = render_page(&markdown, &fallback_title, &request.options.html);
        progress.set(0.8);

        fs::write(request.output, page).map_err(|e| ConvertError::io(request.output, e))
    }
}

/// Parser/renderer settings: CommonMark plus the GitHub extensions.
fn comrak_options() -> Options<'static> {
    let mut options = Options::default();

    let ext = &mut options.extension;
    ext.table = true;
    ext.tasklist = true;
    ext.strikethrough = true;
    ext.autolink = true;
    ext.footnotes = true;
    ext.alerts = true;
    ext.header_id_prefix = Some(String::new()); // `id="section-name"` anchors

    let render = &mut options.render;
    render.r#unsafe = true; // keep raw HTML from the source, see type docs
    render.github_pre_lang = true; // `<pre lang="rust">` like GitHub
    render.tasklist_classes = true; // classes used by the stylesheet

    options
}

/// Renders `markdown` into a complete HTML document.
pub(crate) fn render_page(markdown: &str, fallback_title: &str, options: &HtmlOptions) -> String {
    let arena = Arena::new();
    let comrak_options = comrak_options();
    let root = parse_document(&arena, markdown, &comrak_options);

    let title = root
        .descendants()
        .find(|node| matches!(node.data().value, NodeValue::Heading(_)))
        .map(|heading| heading.collect_text())
        .filter(|text| !text.trim().is_empty())
        .unwrap_or_else(|| fallback_title.to_owned());

    let mut body = String::with_capacity(markdown.len() * 2);
    format_html(root, &comrak_options, &mut body).expect("formatting into a String cannot fail");

    wrap_in_document(title.trim(), &body, options)
}

/// Builds the `<!DOCTYPE html>` skeleton around the rendered body.
fn wrap_in_document(title: &str, body: &str, options: &HtmlOptions) -> String {
    let (theme_attr, style) = match options.theme.as_attribute() {
        Some(theme) => (
            format!(" data-theme=\"{theme}\""),
            format!("<style>\n{STYLESHEET}</style>\n"),
        ),
        None => (String::new(), String::new()),
    };
    let title = escape_html(title);

    format!(
        "<!DOCTYPE html>\n\
         <html{theme_attr}>\n\
         <head>\n\
         <meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <meta name=\"generator\" content=\"ConvertApp\">\n\
         <title>{title}</title>\n\
         {style}\
         </head>\n\
         <body>\n\
         <article class=\"markdown-body\">\n\
         {body}\
         </article>\n\
         </body>\n\
         </html>\n"
    )
}

/// The file name without extension, used when the document has no heading.
fn file_title(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Document".to_owned())
}

/// Escapes text for use inside HTML element content or attribute values.
fn escape_html(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HtmlTheme;

    fn render(markdown: &str, theme: HtmlTheme) -> String {
        render_page(markdown, "fallback", &HtmlOptions { theme })
    }

    #[test]
    fn title_comes_from_first_heading() {
        let html = render("intro\n\n## Hello *world*\n\n# Later", HtmlTheme::Auto);
        assert!(html.contains("<title>Hello world</title>"), "{html}");
    }

    #[test]
    fn title_falls_back_to_file_name_and_is_escaped() {
        let html = render_page("no heading", "a<b>&c", &HtmlOptions::default());
        assert!(html.contains("<title>a&lt;b&gt;&amp;c</title>"), "{html}");
    }

    #[test]
    fn github_extensions_are_enabled() {
        let html = render(
            "| a | b |\n|---|---|\n| 1 | 2 |\n\n- [x] done\n\n~~gone~~ https://example.com",
            HtmlTheme::Auto,
        );
        assert!(html.contains("<table>"), "{html}");
        assert!(html.contains("type=\"checkbox\""), "{html}");
        assert!(html.contains("<del>gone</del>"), "{html}");
        assert!(html.contains("<a href=\"https://example.com\">"), "{html}");
    }

    #[test]
    fn theme_controls_stylesheet() {
        let styled = render("# x", HtmlTheme::Dark);
        assert!(styled.contains("<html data-theme=\"dark\">"));
        assert!(styled.contains("<style>"));

        let bare = render("# x", HtmlTheme::None);
        assert!(bare.contains("<html>"));
        assert!(!bare.contains("<style>"));
    }

    #[test]
    fn raw_html_is_preserved() {
        let html = render("<p align=\"center\">logo</p>", HtmlTheme::None);
        assert!(html.contains("<p align=\"center\">logo</p>"), "{html}");
    }
}
