//! SVG → raster images using [`resvg`](https://docs.rs/resvg).

use std::fs;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use image::{DynamicImage, RgbaImage};
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg;

use super::image::encode;
use crate::{ConvertError, Converter, Format, Progress, Request, Resize, Result};

/// Largest width/height we render, to keep memory bounded (256 MiB of pixels).
const MAX_SIDE: u32 = 8192;

/// Renders SVG drawings to raster formats.
///
/// The drawing is rasterised directly at the requested size
/// ([`crate::ImageOptions::resize`]), so enlarging stays sharp. Text uses the
/// fonts installed on the system; images referenced with relative paths are
/// resolved next to the SVG file.
#[derive(Debug, Default, Clone, Copy)]
pub struct SvgConverter;

impl Converter for SvgConverter {
    fn id(&self) -> &'static str {
        "svg"
    }

    fn inputs(&self) -> &[Format] {
        &[Format::Svg]
    }

    fn outputs(&self, input: Format) -> Vec<Format> {
        if input != Format::Svg {
            return Vec::new();
        }
        // Raster targets only: a rasterised PDF would lose the vector quality
        // users expect from an SVG → PDF conversion.
        encode::OUTPUTS
            .iter()
            .copied()
            .filter(|&f| f != Format::Pdf)
            .collect()
    }

    fn convert(&self, request: &Request<'_>, progress: &dyn Progress) -> Result<()> {
        let options = &request.options.image;
        let image = render(request.input, options.resize)?;
        progress.set(0.6);
        encode::write(image, request.target, options, request.output)
    }
}

/// Parses and rasterises the SVG at `path` at the size chosen by `resize`
/// (relative to the drawing's own size).
pub(crate) fn render(path: &Path, resize: Resize) -> Result<DynamicImage> {
    let data = fs::read(path).map_err(|e| ConvertError::io(path, e))?;
    let options = usvg::Options {
        resources_dir: path.parent().map(Path::to_path_buf),
        fontdb: system_fonts(),
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_data(&data, &options).map_err(|e| ConvertError::decode(path, e))?;

    let base = tree.size();
    let (base_w, base_h) = (
        base.width().ceil().max(1.0) as u32,
        base.height().ceil().max(1.0) as u32,
    );
    let (width, height) = resize.apply(base_w, base_h, true);
    if width > MAX_SIDE || height > MAX_SIDE {
        return Err(ConvertError::decode(
            path,
            format!("rendered size {width}×{height} exceeds the {MAX_SIDE}px limit"),
        ));
    }

    let mut pixmap = Pixmap::new(width, height)
        .ok_or_else(|| ConvertError::decode(path, "cannot allocate the render target"))?;
    let transform =
        Transform::from_scale(width as f32 / base.width(), height as f32 / base.height());
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    Ok(DynamicImage::ImageRgba8(to_rgba_image(&pixmap)))
}

/// tiny-skia stores premultiplied alpha; image formats expect straight alpha.
fn to_rgba_image(pixmap: &Pixmap) -> RgbaImage {
    let mut raw = Vec::with_capacity(pixmap.pixels().len() * 4);
    for pixel in pixmap.pixels() {
        let color = pixel.demultiply();
        raw.extend_from_slice(&[color.red(), color.green(), color.blue(), color.alpha()]);
    }
    RgbaImage::from_raw(pixmap.width(), pixmap.height(), raw)
        .expect("buffer length matches the pixmap size")
}

/// System fonts, loaded once per process (scanning the font folders takes a
/// noticeable fraction of a second).
fn system_fonts() -> Arc<usvg::fontdb::Database> {
    static FONTS: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut db = usvg::fontdb::Database::new();
            db.load_system_fonts();
            Arc::new(db)
        })
        .clone()
}
