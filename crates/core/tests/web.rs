//! HTML/Markdown → PDF/PNG through a real headless Edge/Chrome.
//!
//! Skipped (with a notice) when no Chromium browser is installed.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

use convertapp_core::{
    ConvertOptions, Format, Job, NoProgress, PaperSize, Registry, WebOptions, browser, convert,
};

/// Browser tests run one at a time: the leftover-process check would
/// otherwise see browsers that parallel tests are still using.
static BROWSER_LOCK: Mutex<()> = Mutex::new(());

/// Returns a guard serialising browser tests, or `None` (skip) when no
/// browser is installed.
fn browser_guard() -> Option<MutexGuard<'static, ()>> {
    if browser::locate().is_none() {
        eprintln!("skipped: no Edge/Chrome installed");
        return None;
    }
    Some(BROWSER_LOCK.lock().unwrap_or_else(|e| e.into_inner()))
}

fn run(input: &Path, target: Format, web: WebOptions) -> PathBuf {
    let options = ConvertOptions {
        web,
        ..ConvertOptions::default()
    };
    let job = Job::new(input, target).with_options(options);
    convert(&Registry::with_defaults(), &job, &NoProgress)
        .unwrap_or_else(|e| panic!("{} -> {target}: {e}", input.display()))
}

/// A page with a red 100×100 image (relative path) in the top-left corner,
/// a Vietnamese heading and a 3000 px tall block, so it scrolls.
fn tall_page() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    image::RgbImage::from_pixel(100, 100, image::Rgb([255, 0, 0]))
        .save(dir.path().join("red.png"))
        .unwrap();
    let html = dir.path().join("trang dài.html");
    fs::write(
        &html,
        r#"<!doctype html><html><head><meta charset="utf-8"><title>Thử</title>
<style>body{margin:0;background:#fff} img{display:block} .tall{height:3000px;background:linear-gradient(#eef,#cce)}</style>
</head><body><img src="red.png" width="100" height="100"><h1>Tiêu đề tiếng Việt</h1><div class="tall"></div></body></html>"#,
    )
    .unwrap();
    (dir, html)
}

/// Extracts the first `/MediaBox [x0 y0 x1 y1]` width and height from a PDF.
fn media_box(pdf: &[u8]) -> Option<(f64, f64)> {
    let text = String::from_utf8_lossy(pdf);
    let start = text.find("/MediaBox [")? + "/MediaBox [".len();
    let numbers: Vec<f64> = text[start..]
        .split(']')
        .next()?
        .split_whitespace()
        .filter_map(|n| n.parse().ok())
        .collect();
    match numbers[..] {
        [x0, y0, x1, y1] => Some((x1 - x0, y1 - y0)),
        _ => None,
    }
}

#[test]
fn html_to_pdf_uses_paper_size_and_orientation() {
    let Some(_guard) = browser_guard() else {
        return;
    };
    let (_dir, html) = tall_page();

    let started = Instant::now();
    let portrait = fs::read(run(&html, Format::Pdf, WebOptions::default())).unwrap();
    eprintln!("html -> pdf took {:?}", started.elapsed());
    assert!(portrait.starts_with(b"%PDF-"));
    let (w, h) = media_box(&portrait).expect("MediaBox in PDF");
    assert!(
        (w - 595.0).abs() < 2.0 && (h - 842.0).abs() < 2.0,
        "A4 portrait, got {w}x{h}"
    );

    let landscape = fs::read(run(
        &html,
        Format::Pdf,
        WebOptions {
            paper: PaperSize::Letter,
            landscape: true,
            ..WebOptions::default()
        },
    ))
    .unwrap();
    let (w, h) = media_box(&landscape).expect("MediaBox in PDF");
    assert!(
        (w - 792.0).abs() < 2.0 && (h - 612.0).abs() < 2.0,
        "Letter landscape, got {w}x{h}"
    );
}

#[test]
fn html_to_png_captures_the_whole_page_with_relative_images() {
    let Some(_guard) = browser_guard() else {
        return;
    };
    let (_dir, html) = tall_page();

    let png = run(
        &html,
        Format::Png,
        WebOptions {
            viewport_width: 800,
            ..WebOptions::default()
        },
    );

    let image = image::open(&png).unwrap().to_rgb8();
    assert_eq!(image.width(), 800, "window width");
    assert!(
        image.height() >= 3000,
        "full page, got height {}",
        image.height()
    );
    let pixel = image.get_pixel(10, 10).0;
    assert!(
        pixel[0] > 240 && pixel[1] < 20 && pixel[2] < 20,
        "red image loaded: {pixel:?}"
    );
}

#[test]
fn markdown_to_pdf_uses_the_browser_and_cleans_up() {
    let Some(_guard) = browser_guard() else {
        return;
    };
    let registry = Registry::with_defaults();
    assert_eq!(
        registry.find(Format::Markdown, Format::Pdf).unwrap().id(),
        "browser",
        "browser is preferred over Pandoc + Typst"
    );

    let dir = tempfile::tempdir().unwrap();
    let md = dir.path().join("ghi chú.md");
    fs::write(&md, "# Ghi chú\n\n| A | B |\n|---|---|\n| 1 | 2 |\n").unwrap();

    let pdf = run(&md, Format::Pdf, WebOptions::default());
    assert!(fs::read(&pdf).unwrap().starts_with(b"%PDF-"));

    // Only the source and the PDF remain: the temporary HTML page is gone.
    let mut names: Vec<String> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["ghi chú.md", "ghi chú.pdf"]);
}

#[test]
fn markdown_to_png_works() {
    let Some(_guard) = browser_guard() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let md = dir.path().join("a.md");
    fs::write(&md, "# Xin chào\n\nNội dung.").unwrap();

    let png = run(&md, Format::Png, WebOptions::default());
    let image = image::open(png).unwrap();
    assert_eq!(image.width(), 1280);
}

#[test]
fn no_browser_processes_are_left_behind() {
    let Some(_guard) = browser_guard() else {
        return;
    };
    let (_dir, html) = tall_page();
    run(&html, Format::Png, WebOptions::default());

    // Any headless instance we started used a profile named convertapp-browser-*.
    #[cfg(windows)]
    {
        let exe = browser::locate().unwrap();
        let exe_name = exe.file_name().unwrap().to_string_lossy().into_owned();
        let output = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "(Get-CimInstance Win32_Process -Filter \"Name='{exe_name}'\" | \
                     Where-Object {{ $_.CommandLine -like '*convertapp-browser-*' }}).Count"
                ),
            ])
            .output()
            .unwrap();
        let count = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        assert!(
            count.is_empty() || count == "0",
            "leftover browser processes: {count}"
        );
    }
}
