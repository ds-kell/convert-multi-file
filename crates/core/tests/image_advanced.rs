//! Phase 5 features: WebP quality, EXIF orientation, resizing, SVG rendering
//! and images → PDF (single and merged).

use std::fs;
use std::path::{Path, PathBuf};

use convertapp_core::{
    ConvertError, ConvertOptions, Format, ImageOptions, Job, NoProgress, PdfPageSize, Registry,
    Resize, convert, merge,
};
use image::{DynamicImage, ImageFormat, ImageReader, Rgb, RgbImage};

/// A photo-like RGB image: a smooth gradient plus pseudo-random sensor noise.
/// Real photos compress poorly losslessly because of such noise, which is
/// what makes lossy WebP worthwhile.
fn photo(width: u32, height: u32) -> RgbImage {
    let mut state: u32 = 0x9E37_79B9;
    let mut noise = move || {
        // xorshift32: deterministic, good enough for test noise.
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        (state % 48) as u8
    };
    RgbImage::from_fn(width, height, |x, y| {
        let gx = (x * 200 / width.max(1)) as u8;
        let gy = (y * 200 / height.max(1)) as u8;
        Rgb([gx + noise(), gy + noise(), 120 + noise()])
    })
}

fn save(dir: &Path, name: &str, image: &DynamicImage, format: ImageFormat) -> PathBuf {
    let path = dir.join(name);
    image.save_with_format(&path, format).unwrap();
    path
}

fn open(path: &Path) -> (Option<ImageFormat>, DynamicImage) {
    let reader = ImageReader::open(path)
        .unwrap()
        .with_guessed_format()
        .unwrap();
    let format = reader.format();
    (format, reader.decode().unwrap())
}

fn with_image_options(image: ImageOptions) -> ConvertOptions {
    ConvertOptions {
        image,
        ..ConvertOptions::default()
    }
}

fn run(input: &Path, target: Format, image: ImageOptions) -> PathBuf {
    let job = Job::new(input, target).with_options(with_image_options(image));
    convert(&Registry::with_defaults(), &job, &NoProgress)
        .unwrap_or_else(|e| panic!("{} -> {target}: {e}", input.display()))
}

/// Inserts an EXIF APP1 segment with the given orientation right after the
/// JPEG start-of-image marker, like a phone camera does.
fn add_exif_orientation(jpeg: &[u8], orientation: u8) -> Vec<u8> {
    #[rustfmt::skip]
    let payload: Vec<u8> = [
        b"Exif\0\0".as_slice(),
        // TIFF header: little endian, magic 42, first IFD at offset 8.
        &[0x49, 0x49, 0x2A, 0x00, 0x08, 0x00, 0x00, 0x00],
        // One IFD entry: tag 0x0112 (Orientation), type SHORT, count 1, value.
        &[0x01, 0x00],
        &[0x12, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, orientation, 0x00, 0x00, 0x00],
        // No next IFD.
        &[0x00, 0x00, 0x00, 0x00],
    ]
    .concat();
    let length = u16::try_from(payload.len() + 2).unwrap().to_be_bytes();

    let mut out = Vec::with_capacity(jpeg.len() + payload.len() + 4);
    out.extend_from_slice(&jpeg[..2]); // SOI
    out.extend_from_slice(&[0xFF, 0xE1, length[0], length[1]]);
    out.extend_from_slice(&payload);
    out.extend_from_slice(&jpeg[2..]);
    out
}

// ---------- WebP ----------

#[test]
fn lossy_webp_is_smaller_than_lossless_and_quality_matters() {
    let dir = tempfile::tempdir().unwrap();
    let source = save(
        dir.path(),
        "photo.png",
        &DynamicImage::ImageRgb8(photo(320, 240)),
        ImageFormat::Png,
    );

    let size_of = |options: ImageOptions| {
        fs::metadata(run(&source, Format::Webp, options))
            .unwrap()
            .len()
    };
    let lossless = size_of(ImageOptions {
        webp_lossless: true,
        ..ImageOptions::default()
    });
    let q90 = size_of(ImageOptions {
        quality: 90,
        ..ImageOptions::default()
    });
    let q30 = size_of(ImageOptions {
        quality: 30,
        ..ImageOptions::default()
    });

    assert!(q30 < q90, "q30={q30} q90={q90}");
    assert!(q90 < lossless, "q90={q90} lossless={lossless}");
}

#[test]
fn webp_keeps_transparency() {
    let dir = tempfile::tempdir().unwrap();
    let mut rgba = image::RgbaImage::from_pixel(16, 16, image::Rgba([255, 0, 0, 255]));
    rgba.put_pixel(0, 0, image::Rgba([0, 0, 0, 0]));
    let source = save(
        dir.path(),
        "a.png",
        &DynamicImage::ImageRgba8(rgba),
        ImageFormat::Png,
    );

    let output = run(&source, Format::Webp, ImageOptions::default());
    let (format, image) = open(&output);
    assert_eq!(format, Some(ImageFormat::WebP));
    assert!(image.color().has_alpha());
    assert_eq!(image.to_rgba8().get_pixel(0, 0).0[3], 0);
}

// ---------- EXIF orientation ----------

#[test]
fn exif_orientation_is_applied_to_the_pixels() {
    let dir = tempfile::tempdir().unwrap();
    // A 40×20 landscape JPEG tagged "rotate 90° clockwise" (orientation 6),
    // which is how phones store portrait photos.
    let mut jpeg = Vec::new();
    DynamicImage::ImageRgb8(photo(40, 20))
        .write_to(&mut std::io::Cursor::new(&mut jpeg), ImageFormat::Jpeg)
        .unwrap();
    let source = dir.path().join("phone.jpg");
    fs::write(&source, add_exif_orientation(&jpeg, 6)).unwrap();

    let output = run(
        &source,
        Format::Webp,
        ImageOptions {
            quality: 80,
            ..ImageOptions::default()
        },
    );

    let (_, image) = open(&output);
    assert_eq!(
        (image.width(), image.height()),
        (20, 40),
        "rotated to portrait"
    );
}

// ---------- Resize ----------

#[test]
fn resize_options_are_applied() {
    let dir = tempfile::tempdir().unwrap();
    let source = save(
        dir.path(),
        "big.png",
        &DynamicImage::ImageRgb8(photo(400, 200)),
        ImageFormat::Png,
    );

    let fit = run(
        &source,
        Format::Jpeg,
        ImageOptions {
            resize: Resize::Fit {
                max_width: Some(100),
                max_height: Some(100),
            },
            ..ImageOptions::default()
        },
    );
    assert_eq!(open(&fit).1.width(), 100);
    assert_eq!(open(&fit).1.height(), 50);

    let percent = run(
        &source,
        Format::Bmp,
        ImageOptions {
            resize: Resize::Percent(25),
            ..ImageOptions::default()
        },
    );
    let (_, image) = open(&percent);
    assert_eq!((image.width(), image.height()), (100, 50));
}

// ---------- SVG ----------

fn svg_fixture(dir: &Path) -> PathBuf {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.svg");
    let copy = dir.join("sample.svg");
    fs::copy(source, &copy).unwrap();
    copy
}

#[test]
fn svg_is_detected_and_offers_raster_targets() {
    let dir = tempfile::tempdir().unwrap();
    let svg = svg_fixture(dir.path());
    assert_eq!(Format::detect(&svg).unwrap(), Some(Format::Svg));

    let targets = Registry::with_defaults().targets_for(Format::Svg);
    assert!(targets.contains(&Format::Png));
    assert!(targets.contains(&Format::Webp));
    assert!(!targets.contains(&Format::Pdf));
}

#[test]
fn svg_renders_at_its_size_with_transparency() {
    let dir = tempfile::tempdir().unwrap();
    let output = run(
        &svg_fixture(dir.path()),
        Format::Png,
        ImageOptions::default(),
    );

    let (format, image) = open(&output);
    assert_eq!(format, Some(ImageFormat::Png));
    assert_eq!((image.width(), image.height()), (200, 100));
    let rgba = image.to_rgba8();
    assert_eq!(rgba.get_pixel(20, 20).0, [255, 0, 0, 255], "red rectangle");
    assert_eq!(rgba.get_pixel(110, 5).0[3], 0, "transparent background");
}

#[test]
fn svg_can_be_enlarged_sharply() {
    let dir = tempfile::tempdir().unwrap();
    let output = run(
        &svg_fixture(dir.path()),
        Format::Jpeg,
        ImageOptions {
            resize: Resize::Fit {
                max_width: Some(800),
                max_height: None,
            },
            ..ImageOptions::default()
        },
    );
    let (_, image) = open(&output);
    assert_eq!((image.width(), image.height()), (800, 400));
    // Transparent area flattened onto the white background for JPEG.
    let corner = image.to_rgb8().get_pixel(790, 5).0;
    assert!(corner.iter().all(|&c| c > 245), "{corner:?}");
}

#[test]
fn invalid_svg_is_a_decode_error() {
    let dir = tempfile::tempdir().unwrap();
    let svg = dir.path().join("broken.svg");
    fs::write(&svg, "<svg><not closed").unwrap();

    let job = Job::new(&svg, Format::Png);
    let err = convert(&Registry::with_defaults(), &job, &NoProgress).unwrap_err();
    assert!(matches!(err, ConvertError::Decode { .. }), "{err:?}");
}

// ---------- PDF ----------
// The page count is read from the page tree's `/Count` entry.

#[test]
fn image_to_pdf_writes_a_one_page_document() {
    let dir = tempfile::tempdir().unwrap();
    let source = save(
        dir.path(),
        "scan.png",
        &DynamicImage::ImageRgb8(photo(300, 400)),
        ImageFormat::Png,
    );

    let output = run(&source, Format::Pdf, ImageOptions::default());
    assert_eq!(output.file_name().unwrap(), "scan.pdf");
    let bytes = fs::read(&output).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
    assert!(String::from_utf8_lossy(&bytes).contains("/Count 1"));
}

#[test]
fn image_page_size_uses_the_image_dimensions() {
    let dir = tempfile::tempdir().unwrap();
    let source = save(
        dir.path(),
        "a.png",
        &DynamicImage::ImageRgb8(photo(400, 200)),
        ImageFormat::Png,
    );

    let output = run(
        &source,
        Format::Pdf,
        ImageOptions {
            pdf_page: PdfPageSize::Image,
            ..ImageOptions::default()
        },
    );
    let text = String::from_utf8_lossy(&fs::read(output).unwrap()).into_owned();
    // 400×200 px at 96 DPI = 300×150 pt.
    assert!(
        text.contains("/MediaBox [0 0 300 150]"),
        "{}",
        &text[..text.len().min(600)]
    );
}

#[test]
fn merge_puts_every_image_on_its_own_page() {
    let dir = tempfile::tempdir().unwrap();
    let inputs = vec![
        save(
            dir.path(),
            "1.png",
            &DynamicImage::ImageRgb8(photo(50, 50)),
            ImageFormat::Png,
        ),
        save(
            dir.path(),
            "2.jpg",
            &DynamicImage::ImageRgb8(photo(60, 40)),
            ImageFormat::Jpeg,
        ),
        svg_fixture(dir.path()),
    ];

    let output = merge(
        &inputs,
        Format::Pdf,
        None,
        &ConvertOptions::default(),
        &NoProgress,
    )
    .unwrap();

    assert_eq!(
        output,
        dir.path().join("1.pdf"),
        "named after the first image"
    );
    let text = String::from_utf8_lossy(&fs::read(output).unwrap()).into_owned();
    assert!(text.contains("/Count 3"));
}

#[test]
fn merge_rejects_non_images_without_writing_anything() {
    let dir = tempfile::tempdir().unwrap();
    let png = save(
        dir.path(),
        "a.png",
        &DynamicImage::ImageRgb8(photo(8, 8)),
        ImageFormat::Png,
    );
    let md = dir.path().join("notes.md");
    fs::write(&md, "# hi").unwrap();

    let err = merge(
        &[png, md],
        Format::Pdf,
        None,
        &ConvertOptions::default(),
        &NoProgress,
    )
    .unwrap_err();
    assert!(matches!(err, ConvertError::Unsupported { .. }), "{err:?}");
    assert!(!dir.path().join("a.pdf").exists());

    let empty = merge(
        &[],
        Format::Pdf,
        None,
        &ConvertOptions::default(),
        &NoProgress,
    );
    assert!(matches!(empty, Err(ConvertError::NoInput)));
}
