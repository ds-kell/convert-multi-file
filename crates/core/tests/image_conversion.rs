//! End-to-end tests for the image converter, going through the public
//! [`convertapp_core::convert`] pipeline.
//!
//! Source images are generated at test time (see [`write_source`]) instead of
//! being stored as binary fixtures: every format is produced from the same
//! known pixels, which keeps the repository small and lets us assert exact
//! sizes and colours.

use std::fs;
use std::path::{Path, PathBuf};

use convertapp_core::{ConvertError, Format, Job, NoProgress, Registry, convert};
use image::{DynamicImage, ImageFormat, ImageReader, Rgba, RgbaImage};

const IMAGE_FORMATS: &[Format] = &[
    Format::Png,
    Format::Jpeg,
    Format::Webp,
    Format::Bmp,
    Format::Gif,
    Format::Tiff,
    Format::Ico,
];

const WIDTH: u32 = 40;
const HEIGHT: u32 = 30;

/// A gradient with a fully transparent top-left 10×10 square.
fn sample_pixels(width: u32, height: u32) -> RgbaImage {
    RgbaImage::from_fn(width, height, |x, y| {
        let alpha = if x < 10 && y < 10 { 0 } else { 255 };
        Rgba([(x * 6) as u8, (y * 8) as u8, 128, alpha])
    })
}

/// Writes `pixels` as `name` in `format` and returns the path.
fn write_source(dir: &Path, name: &str, format: Format, pixels: &RgbaImage) -> PathBuf {
    let path = dir.join(name);
    let image = DynamicImage::ImageRgba8(pixels.clone());
    let image = match format {
        // These encoders do not take RGBA input.
        Format::Jpeg | Format::Bmp => DynamicImage::ImageRgb8(image.to_rgb8()),
        _ => image,
    };
    image
        .save_with_format(&path, to_image_format(format))
        .expect("write source image");
    path
}

fn to_image_format(format: Format) -> ImageFormat {
    match format {
        Format::Png => ImageFormat::Png,
        Format::Jpeg => ImageFormat::Jpeg,
        Format::Webp => ImageFormat::WebP,
        Format::Bmp => ImageFormat::Bmp,
        Format::Gif => ImageFormat::Gif,
        Format::Tiff => ImageFormat::Tiff,
        Format::Ico => ImageFormat::Ico,
        other => panic!("{other} is not an image format"),
    }
}

/// Opens `path`, sniffing the format from its content.
fn open(path: &Path) -> (Option<ImageFormat>, DynamicImage) {
    let reader = ImageReader::open(path)
        .unwrap()
        .with_guessed_format()
        .unwrap();
    let format = reader.format();
    (format, reader.decode().unwrap())
}

/// Lists files in `dir`, to make sure no temporary files are left behind.
fn file_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn converts_every_image_format_to_every_other() {
    let registry = Registry::with_defaults();
    let pixels = sample_pixels(WIDTH, HEIGHT);

    for &source_format in IMAGE_FORMATS {
        let dir = tempfile::tempdir().unwrap();
        let source = write_source(
            dir.path(),
            &format!("src.{}", source_format.extension()),
            source_format,
            &pixels,
        );

        for &target in IMAGE_FORMATS.iter().filter(|&&t| t != source_format) {
            let job = Job::new(&source, target);
            let output = convert(&registry, &job, &NoProgress)
                .unwrap_or_else(|e| panic!("{source_format} -> {target}: {e}"));

            assert_eq!(output.extension().unwrap(), target.extension());
            let (detected, image) = open(&output);
            assert_eq!(
                detected,
                Some(to_image_format(target)),
                "{source_format} -> {target}"
            );
            assert_eq!((image.width(), image.height()), (WIDTH, HEIGHT));
        }
    }
}

#[test]
fn transparent_pixels_become_background_in_jpeg() {
    let dir = tempfile::tempdir().unwrap();
    let source = write_source(
        dir.path(),
        "alpha.png",
        Format::Png,
        &sample_pixels(WIDTH, HEIGHT),
    );

    let output = convert(
        &Registry::with_defaults(),
        &Job::new(&source, Format::Jpeg),
        &NoProgress,
    )
    .unwrap();

    let (_, image) = open(&output);
    let corner = image.to_rgb8().get_pixel(2, 2).0;
    // JPEG is lossy, so allow a small tolerance around pure white.
    assert!(
        corner.iter().all(|&c| c > 245),
        "expected white, got {corner:?}"
    );
}

#[test]
fn large_images_are_scaled_down_for_ico() {
    let dir = tempfile::tempdir().unwrap();
    let source = write_source(dir.path(), "big.png", Format::Png, &sample_pixels(600, 300));

    let output = convert(
        &Registry::with_defaults(),
        &Job::new(&source, Format::Ico),
        &NoProgress,
    )
    .unwrap();

    let (_, image) = open(&output);
    assert_eq!((image.width(), image.height()), (256, 128));
}

#[test]
fn existing_files_are_never_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let source = write_source(dir.path(), "photo.png", Format::Png, &sample_pixels(8, 8));
    let registry = Registry::with_defaults();
    let job = Job::new(&source, Format::Bmp);

    let first = convert(&registry, &job, &NoProgress).unwrap();
    let second = convert(&registry, &job, &NoProgress).unwrap();

    assert_eq!(first.file_name().unwrap(), "photo.bmp");
    assert_eq!(second.file_name().unwrap(), "photo (1).bmp");
    assert_eq!(
        file_names(dir.path()),
        ["photo (1).bmp", "photo.bmp", "photo.png"]
    );
}

#[test]
fn output_dir_is_created_when_missing() {
    let dir = tempfile::tempdir().unwrap();
    let source = write_source(dir.path(), "photo.png", Format::Png, &sample_pixels(8, 8));
    let out_dir = dir.path().join("nested").join("out");

    let job = Job::new(&source, Format::Gif).with_output_dir(&out_dir);
    let output = convert(&Registry::with_defaults(), &job, &NoProgress).unwrap();

    assert_eq!(output, out_dir.join("photo.gif"));
}

#[test]
fn content_wins_over_a_wrong_extension() {
    let dir = tempfile::tempdir().unwrap();
    // A JPEG file that was renamed to .png.
    let jpeg = write_source(dir.path(), "real.jpg", Format::Jpeg, &sample_pixels(8, 8));
    let lying = dir.path().join("liar.png");
    fs::rename(&jpeg, &lying).unwrap();

    // Converting to PNG must work, because the file is really a JPEG.
    let output = convert(
        &Registry::with_defaults(),
        &Job::new(&lying, Format::Png),
        &NoProgress,
    )
    .unwrap();
    assert_eq!(output.file_name().unwrap(), "liar (1).png");
    assert_eq!(open(&output).0, Some(ImageFormat::Png));
}

#[test]
fn reports_unknown_formats() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("notes.xyz");
    fs::write(&source, "plain text").unwrap();

    let err = convert(
        &Registry::with_defaults(),
        &Job::new(&source, Format::Png),
        &NoProgress,
    )
    .unwrap_err();
    assert!(matches!(err, ConvertError::UnknownFormat { .. }), "{err:?}");
}

#[test]
fn rejects_converting_to_the_same_format() {
    let dir = tempfile::tempdir().unwrap();
    let source = write_source(dir.path(), "a.png", Format::Png, &sample_pixels(4, 4));

    let err = convert(
        &Registry::with_defaults(),
        &Job::new(&source, Format::Png),
        &NoProgress,
    )
    .unwrap_err();
    assert!(matches!(err, ConvertError::Unsupported { .. }), "{err:?}");
}

#[test]
fn corrupt_input_fails_without_leaving_files_behind() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("broken.bmp");
    // Valid BMP signature followed by garbage.
    fs::write(&source, b"BM\x00\x01garbage-garbage-garbage").unwrap();

    let err = convert(
        &Registry::with_defaults(),
        &Job::new(&source, Format::Png),
        &NoProgress,
    )
    .unwrap_err();
    assert!(matches!(err, ConvertError::Decode { .. }), "{err:?}");
    assert_eq!(file_names(dir.path()), ["broken.bmp"]);
}

#[test]
fn reports_progress_up_to_completion() {
    use std::sync::Mutex;

    let dir = tempfile::tempdir().unwrap();
    let source = write_source(dir.path(), "a.png", Format::Png, &sample_pixels(8, 8));
    let seen = Mutex::new(Vec::new());
    let progress = |fraction: f32| seen.lock().unwrap().push(fraction);

    convert(
        &Registry::with_defaults(),
        &Job::new(&source, Format::Jpeg),
        &progress,
    )
    .unwrap();

    let seen = seen.into_inner().unwrap();
    assert_eq!(seen.first(), Some(&0.0));
    assert_eq!(seen.last(), Some(&1.0));
    assert!(
        seen.windows(2).all(|w| w[0] <= w[1]),
        "progress must not go backwards: {seen:?}"
    );
}
