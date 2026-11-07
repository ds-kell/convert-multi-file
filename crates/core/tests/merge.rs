//! Merging several files into one: PDFs + images → PDF, images → picture.

use std::fs;
use std::path::{Path, PathBuf};

use convertapp_core::{
    ConvertError, ConvertOptions, Format, ImageOptions, Job, NoProgress, PdfPageSize, Registry,
    StitchDirection, StitchOptions, browser, convert, merge, merge_targets,
};
use image::{DynamicImage, Rgb, RgbImage};
use lopdf::Document;

/// Saves a solid-colour PNG of the given size.
fn png(dir: &Path, name: &str, width: u32, height: u32, color: [u8; 3]) -> PathBuf {
    let path = dir.join(name);
    DynamicImage::ImageRgb8(RgbImage::from_pixel(width, height, Rgb(color)))
        .save(&path)
        .unwrap();
    path
}

/// Options where a PDF page is exactly as large as its image, so page
/// sizes identify which image a page came from.
fn exact_pages() -> ConvertOptions {
    ConvertOptions {
        image: ImageOptions {
            pdf_page: PdfPageSize::Image,
            ..ImageOptions::default()
        },
        ..ConvertOptions::default()
    }
}

/// Widths (in points) of every page of a PDF, in page order.
fn page_widths(pdf: &Path) -> Vec<i64> {
    let document = Document::load(pdf).unwrap();
    document
        .get_pages()
        .into_values()
        .map(|id| {
            let page = document.get_dictionary(id).unwrap();
            let media_box = page.get(b"MediaBox").unwrap().as_array().unwrap();
            media_box[2].as_float().unwrap().round() as i64
        })
        .collect()
}

#[test]
fn pdfs_and_images_are_merged_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let options = exact_pages();
    // Image widths 400/800/1200/1600 px → 300/600/900/1200 pt pages.
    let a = png(dir.path(), "a.png", 400, 100, [255, 0, 0]);
    let b = png(dir.path(), "b.png", 800, 100, [0, 255, 0]);
    let c = png(dir.path(), "c.png", 1200, 100, [0, 0, 255]);
    let d = png(dir.path(), "d.png", 1600, 100, [9, 9, 9]);

    // A two-page PDF (a, b) and a one-page PDF (d).
    let ab = merge(&[a, b], Format::Pdf, None, &options, &NoProgress).unwrap();
    let d_pdf = convert(
        &Registry::with_defaults(),
        &Job::new(&d, Format::Pdf).with_options(options.clone()),
        &NoProgress,
    )
    .unwrap();

    // PDF + image + PDF, in that order.
    let out_dir = dir.path().join("out");
    let merged = merge(
        &[ab, c, d_pdf],
        Format::Pdf,
        Some(&out_dir),
        &options,
        &NoProgress,
    )
    .unwrap();

    assert_eq!(merged, out_dir.join("a.pdf"), "named after the first input");
    assert_eq!(page_widths(&merged), [300, 600, 900, 1200]);
}

#[test]
fn browser_made_pdfs_merge_too() {
    // Real-world PDFs (from Chromium) use features our own writer does not,
    // e.g. object streams and inherited attributes.
    if browser::locate().is_none() {
        eprintln!("skipped: no Edge/Chrome installed");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let mut pdfs = Vec::new();
    for name in ["one", "two"] {
        let html = dir.path().join(format!("{name}.html"));
        fs::write(
            &html,
            format!("<h1>{name}</h1><div style='height:2000px'></div>"),
        )
        .unwrap();
        pdfs.push(
            convert(
                &Registry::with_defaults(),
                &Job::new(&html, Format::Pdf),
                &NoProgress,
            )
            .unwrap(),
        );
    }
    let pages: usize = pdfs
        .iter()
        .map(|p| Document::load(p).unwrap().get_pages().len())
        .sum();

    let merged = merge(
        &pdfs,
        Format::Pdf,
        None,
        &ConvertOptions::default(),
        &NoProgress,
    )
    .unwrap();
    let document = Document::load(&merged).unwrap();
    assert_eq!(document.get_pages().len(), pages);
    let text = document.extract_text(&[1]).unwrap_or_default();
    assert!(
        text.contains("one"),
        "first page comes from the first file: {text:?}"
    );
}

#[test]
fn images_are_stitched_vertically() {
    let dir = tempfile::tempdir().unwrap();
    let top = png(dir.path(), "top.png", 300, 100, [255, 0, 0]);
    let bottom = png(dir.path(), "bottom.png", 600, 400, [0, 0, 255]);

    let joined = merge(
        &[top, bottom],
        Format::Png,
        None,
        &ConvertOptions::default(),
        &NoProgress,
    )
    .unwrap();

    let image = image::open(&joined).unwrap().to_rgb8();
    // The wider image is shrunk to 300 px wide (600×400 → 300×200).
    assert_eq!((image.width(), image.height()), (300, 300));
    assert_eq!(
        image.get_pixel(150, 50).0,
        [255, 0, 0],
        "first image on top"
    );
    assert_eq!(
        image.get_pixel(150, 250).0,
        [0, 0, 255],
        "second image below"
    );
}

#[test]
fn images_are_stitched_horizontally_with_a_gap_as_jpeg() {
    let dir = tempfile::tempdir().unwrap();
    let left = png(dir.path(), "left.png", 100, 100, [255, 0, 0]);
    let right = png(dir.path(), "right.png", 100, 50, [0, 0, 255]);
    let options = ConvertOptions {
        stitch: StitchOptions {
            direction: StitchDirection::Horizontal,
            gap: 20,
            same_size: false,
        },
        ..ConvertOptions::default()
    };

    let joined = merge(&[left, right], Format::Jpeg, None, &options, &NoProgress).unwrap();
    let image = image::open(&joined).unwrap().to_rgb8();
    assert_eq!((image.width(), image.height()), (220, 100));
    // Gap and the area around the shorter image are white in a JPEG.
    let gap = image.get_pixel(110, 50).0;
    assert!(gap.iter().all(|&c| c > 240), "gap should be white: {gap:?}");
}

#[test]
fn svg_and_raster_images_can_be_stitched() {
    let dir = tempfile::tempdir().unwrap();
    let svg = dir.path().join("shape.svg");
    fs::write(
        &svg,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="50"><rect width="200" height="50" fill="#00ff00"/></svg>"##,
    )
    .unwrap();
    let raster = png(dir.path(), "photo.png", 200, 50, [255, 0, 0]);

    let joined = merge(
        &[svg, raster],
        Format::Webp,
        None,
        &ConvertOptions::default(),
        &NoProgress,
    )
    .unwrap();
    let image = image::open(joined).unwrap();
    assert_eq!((image.width(), image.height()), (200, 100));
}

#[test]
fn incompatible_inputs_are_rejected_before_writing() {
    let dir = tempfile::tempdir().unwrap();
    let image = png(dir.path(), "a.png", 10, 10, [0, 0, 0]);
    let pdf = merge(
        &[image.clone(), image.clone()],
        Format::Pdf,
        None,
        &ConvertOptions::default(),
        &NoProgress,
    )
    .unwrap();

    // A PDF cannot be stitched into a picture.
    let err = merge(
        &[image, pdf],
        Format::Png,
        None,
        &ConvertOptions::default(),
        &NoProgress,
    )
    .unwrap_err();
    assert!(matches!(err, ConvertError::Unsupported { .. }), "{err:?}");
    assert!(!dir.path().join("a (1).png").exists());
}

#[test]
fn merge_targets_reflect_the_selection() {
    assert!(merge_targets(&[Format::Png, Format::Jpeg]).contains(&Format::Webp));
    assert_eq!(merge_targets(&[Format::Pdf, Format::Png]), [Format::Pdf]);
    assert!(merge_targets(&[Format::Markdown, Format::Pdf]).is_empty());
}
