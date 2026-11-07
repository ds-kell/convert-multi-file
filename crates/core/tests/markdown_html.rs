//! End-to-end tests for Markdown ↔ HTML through the public pipeline, using the
//! real-world style samples in `tests/fixtures/`.

use std::fs;
use std::path::{Path, PathBuf};

use convertapp_core::{
    ConvertOptions, Format, HtmlOptions, HtmlTheme, Job, NoProgress, Registry, convert,
};

/// Copies a fixture into a fresh temp dir so outputs do not pollute the repo.
fn fixture(name: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let copy = dir.path().join(name);
    fs::copy(source, &copy).unwrap();
    (dir, copy)
}

fn convert_to(input: &Path, target: Format) -> String {
    let output = convert(
        &Registry::with_defaults(),
        &Job::new(input, target),
        &NoProgress,
    )
    .unwrap_or_else(|e| panic!("converting {} to {target}: {e}", input.display()));
    fs::read_to_string(output).unwrap()
}

#[test]
fn markdown_is_offered_html_and_back() {
    let registry = Registry::with_defaults();
    // Built-in pairs; Word/PDF targets also appear when Pandoc is installed.
    assert!(
        registry
            .targets_for(Format::Markdown)
            .contains(&Format::Html)
    );
    assert!(
        registry
            .targets_for(Format::Html)
            .contains(&Format::Markdown)
    );
}

#[test]
fn readme_starting_with_html_is_still_markdown() {
    // sample.md starts with `<p align="center">`, which content sniffing
    // would call HTML. The .md extension must win.
    let (_dir, md) = fixture("sample.md");
    assert_eq!(Format::detect(&md).unwrap(), Some(Format::Markdown));
}

#[test]
fn markdown_to_html_produces_a_styled_page() {
    let (_dir, md) = fixture("sample.md");
    let html = convert_to(&md, Format::Html);

    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("<meta charset=\"utf-8\">"));
    assert!(
        html.contains("<title>Hướng dẫn sử dụng</title>"),
        "Vietnamese title"
    );
    assert!(html.contains("data-theme=\"auto\""));
    assert!(html.contains("<table>"));
    assert!(html.contains("<td>png</td>"));
    assert!(html.contains("type=\"checkbox\""));
    assert!(html.contains("<pre lang=\"rust\">"));
    assert!(
        html.contains("<img src=\"images/logo.png\" alt=\"Logo\""),
        "relative image kept"
    );
    assert!(html.contains("<del>gạch ngang</del>"));
    assert!(html.contains("markdown-alert"), "GitHub alert rendered");
    assert!(html.contains("footnote"));
}

#[test]
fn markdown_theme_option_is_applied() {
    let (_dir, md) = fixture("sample.md");
    let options = ConvertOptions {
        html: HtmlOptions {
            theme: HtmlTheme::None,
        },
        ..ConvertOptions::default()
    };
    let job = Job::new(&md, Format::Html).with_options(options);
    let output = convert(&Registry::with_defaults(), &job, &NoProgress).unwrap();

    let html = fs::read_to_string(output).unwrap();
    assert!(!html.contains("<style>"));
}

#[test]
fn html_to_markdown_keeps_content_and_drops_noise() {
    let (_dir, page) = fixture("sample.html");
    let md = convert_to(&page, Format::Markdown);

    assert!(md.contains("# Tiêu đề chính"), "{md}");
    assert!(md.contains("**chữ đậm**"), "{md}");
    assert!(md.contains("[liên kết](https://example.com/docs)"), "{md}");
    assert!(md.contains("1. Bước một"), "{md}");
    assert!(md.contains("const x = 1;"), "{md}");
    assert!(md.contains("> Trích dẫn quan trọng."), "{md}");
    assert!(md.contains("![Ảnh minh hoạ](images/photo.jpg)"), "{md}");
    assert!(!md.contains("console.log"), "script must be dropped: {md}");
    assert!(!md.contains("font-family"), "style must be dropped: {md}");
}

#[test]
fn legacy_encoded_html_is_decoded() {
    let dir = tempfile::tempdir().unwrap();
    let page = dir.path().join("old.htm");
    // "Café" in Windows-1252, declared via <meta charset>.
    fs::write(
        &page,
        b"<html><head><meta charset=\"windows-1252\"></head><body><p>Caf\xE9</p></body></html>",
    )
    .unwrap();

    assert_eq!(convert_to(&page, Format::Markdown), "Café\n");
}

#[test]
fn round_trip_keeps_the_main_content() {
    let (dir, md) = fixture("sample.md");
    let html_path = convert(
        &Registry::with_defaults(),
        &Job::new(&md, Format::Html),
        &NoProgress,
    )
    .unwrap();
    // Give the HTML its own name so the round trip does not collide with sample.md.
    let renamed = dir.path().join("round.html");
    fs::rename(&html_path, &renamed).unwrap();

    let back = convert_to(&renamed, Format::Markdown);
    for expected in [
        "Hướng dẫn sử dụng",
        "Bảng định dạng",
        "Xin chào",
        "Chuyển ảnh",
    ] {
        assert!(back.contains(expected), "missing {expected:?} in:\n{back}");
    }
    assert!(
        !back.contains("--fg"),
        "stylesheet must not leak into Markdown"
    );
}
