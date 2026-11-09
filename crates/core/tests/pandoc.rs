//! End-to-end tests for the Pandoc-based converters (Word, Markdown → PDF).
//!
//! Pandoc and Typst are external programs. When they are not installed, each
//! test prints a notice and returns early instead of failing, so the suite
//! still passes on a fresh machine or CI runner.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use convertapp_core::tools::{PANDOC, TYPST, Tool};
use convertapp_core::{ConvertOptions, Format, Job, NoProgress, Registry, convert};

/// `true` if all `tools` are installed; otherwise logs why the test is skipped.
fn have(tools: &[Tool]) -> bool {
    let missing: Vec<&str> = tools
        .iter()
        .filter(|t| t.locate().is_none())
        .map(|t| t.name)
        .collect();
    if missing.is_empty() {
        true
    } else {
        eprintln!("skipped: {} not installed", missing.join(", "));
        false
    }
}

/// Markdown with the features the acceptance criteria care about.
const SAMPLE_MD: &str = "\
# Báo cáo tháng

Đoạn văn **tiếng Việt** có dấu: ầ ẩ ỡ ự Đ đ.

| Hạng mục | Số lượng |
|---|---|
| Ảnh | 12 |
| Tài liệu | 3 |

![Biểu đồ](chart.png)
";

/// Writes `SAMPLE_MD` plus the image it references into a temp dir.
fn sample_markdown() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    image::RgbImage::from_pixel(40, 20, image::Rgb([30, 120, 200]))
        .save(dir.path().join("chart.png"))
        .unwrap();
    let md = dir.path().join("report.md");
    fs::write(&md, SAMPLE_MD).unwrap();
    (dir, md)
}

fn run(input: &Path, target: Format) -> PathBuf {
    let job = Job::new(input, target).with_options(ConvertOptions::default());
    convert(&Registry::with_defaults(), &job, &NoProgress)
        .unwrap_or_else(|e| panic!("{} -> {target}: {e}", input.display()))
}

/// Reads one entry of a .docx (which is a ZIP archive).
fn docx_entry(docx: &Path, name: &str) -> String {
    let mut archive = zip::ZipArchive::new(fs::File::open(docx).unwrap()).unwrap();
    let mut text = String::new();
    archive
        .by_name(name)
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    text
}

fn docx_has_media(docx: &Path) -> bool {
    let archive = zip::ZipArchive::new(fs::File::open(docx).unwrap()).unwrap();
    archive
        .file_names()
        .any(|name| name.is_ok_and(|name| name.starts_with("word/media/")))
}

#[test]
fn markdown_to_docx_keeps_vietnamese_table_and_image() {
    if !have(&[PANDOC]) {
        return;
    }
    let (_dir, md) = sample_markdown();
    let docx = run(&md, Format::Docx);

    assert_eq!(Format::detect(&docx).unwrap(), Some(Format::Docx));
    let body = docx_entry(&docx, "word/document.xml");
    assert!(body.contains("Báo cáo tháng"), "heading text");
    assert!(body.contains("ầ ẩ ỡ ự Đ đ"), "Vietnamese diacritics");
    assert!(body.contains("<w:tbl>"), "a real Word table");
    assert!(body.contains("Tài liệu"));
    assert!(docx_has_media(&docx), "picture embedded");
}

#[test]
fn docx_to_markdown_extracts_pictures() {
    if !have(&[PANDOC]) {
        return;
    }
    let (dir, md) = sample_markdown();
    let docx = run(&md, Format::Docx);
    // Remove the original so the round trip does not collide with report.md.
    fs::remove_file(&md).unwrap();

    let back = fs::read_to_string(run(&docx, Format::Markdown)).unwrap();
    assert!(back.contains("# Báo cáo tháng"), "{back}");
    assert!(back.contains("| Ảnh"), "table kept as GFM: {back}");
    assert!(back.contains("report_files/media/"), "image link: {back}");
    assert!(
        dir.path().join("report_files").is_dir(),
        "media folder created"
    );
}

#[test]
fn docx_to_html_is_self_contained() {
    if !have(&[PANDOC]) {
        return;
    }
    let (_dir, md) = sample_markdown();
    let docx = run(&md, Format::Docx);
    let html = fs::read_to_string(run(&docx, Format::Html)).unwrap();

    assert!(html.contains("<table"));
    assert!(
        html.contains("data:image/png;base64,"),
        "picture embedded in the page"
    );
    assert!(html.contains("<title>report</title>"));
}

#[test]
fn html_to_docx_works() {
    if !have(&[PANDOC]) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let page = dir.path().join("page.html");
    fs::write(&page, "<h1>Trang web</h1><p>Nội dung <em>nghiêng</em>.</p>").unwrap();

    let docx = run(&page, Format::Docx);
    assert!(docx_entry(&docx, "word/document.xml").contains("Trang web"));
}

#[test]
fn markdown_to_pdf_uses_typst() {
    if !have(&[PANDOC, TYPST]) {
        return;
    }
    let (_dir, md) = sample_markdown();
    let pdf = run(&md, Format::Pdf);

    let bytes = fs::read(&pdf).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
    assert!(bytes.len() > 1000);
}

#[test]
fn pandoc_errors_are_reported() {
    if !have(&[PANDOC]) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    // A .docx that is not a ZIP archive: Pandoc refuses it.
    let broken = dir.path().join("broken.docx");
    fs::write(&broken, "not a word document").unwrap();

    let job = Job::new(&broken, Format::Markdown);
    let result = convert(&Registry::with_defaults(), &job, &NoProgress);
    // Detection by extension says Docx; Pandoc then fails cleanly.
    let err = result.expect_err("broken file must not convert");
    assert!(err.to_string().contains("Pandoc failed"), "{err}");
    assert_eq!(
        fs::read_dir(dir.path()).unwrap().count(),
        1,
        "no output or temp file left behind"
    );
}

#[test]
fn registry_offers_word_targets_only_when_pandoc_is_installed() {
    let registry = Registry::with_defaults();
    let md_targets = registry.targets_for(Format::Markdown);
    if PANDOC.locate().is_some() {
        assert!(md_targets.contains(&Format::Docx));
        assert!(
            registry
                .locked_targets(&[Format::Markdown])
                .iter()
                .all(|l| l.format != Format::Docx)
        );
    } else {
        assert!(!md_targets.contains(&Format::Docx));
        // The hint is the install command of this OS (winget, brew, apt…),
        // or the download page where there is none.
        let hint = PANDOC.install.for_this_os().unwrap_or(PANDOC.homepage);
        let locked = registry.locked_targets(&[Format::Markdown]);
        assert!(
            locked
                .iter()
                .any(|l| l.format == Format::Docx && l.reason.contains(hint))
        );
    }
}
