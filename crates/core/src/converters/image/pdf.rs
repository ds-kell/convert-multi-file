//! Placing raster images on PDF pages with [`pdf_writer`].
//!
//! Each image gets its own page. Opaque images are embedded as JPEG
//! (`DCTDecode`, small); images with transparency are embedded losslessly
//! (`FlateDecode`) with a separate alpha mask (`SMask`).

use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageError};
use miniz_oxide::deflate::{CompressionLevel, compress_to_vec_zlib};
use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect, Ref, TextStr};

use crate::{ImageOptions, PdfPageSize};

/// A4 in PDF points (1 pt = 1/72 inch).
const A4: (f32, f32) = (595.28, 841.89);
/// Margin around images on A4 pages: 1/2 inch.
const MARGIN: f32 = 36.0;
/// Points per pixel, assuming the usual 96 DPI screen resolution.
const PT_PER_PX: f32 = 72.0 / 96.0;
/// Name under which each page refers to its image.
const IMAGE_NAME: Name<'static> = Name(b"Im1");

/// Builds a PDF one image at a time. Images are compressed as they are
/// added, so only the encoded data is kept in memory.
pub(crate) struct PdfBuilder {
    pdf: Pdf,
    catalog: Ref,
    page_tree: Ref,
    next_ref: i32,
    pages: Vec<Ref>,
    page_size: PdfPageSize,
    quality: u8,
}

impl std::fmt::Debug for PdfBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PdfBuilder")
            .field("pages", &self.pages.len())
            .finish_non_exhaustive()
    }
}

impl PdfBuilder {
    /// Starts an empty document using the page layout and JPEG quality from
    /// `options`.
    pub(crate) fn new(options: &ImageOptions) -> Self {
        Self {
            pdf: Pdf::new(),
            catalog: Ref::new(1),
            page_tree: Ref::new(2),
            next_ref: 3,
            pages: Vec::new(),
            page_size: options.pdf_page,
            quality: options.quality.clamp(1, 100),
        }
    }

    /// Adds `image` on a new page.
    pub(crate) fn add_image(&mut self, image: &DynamicImage) -> Result<(), ImageError> {
        let (px_w, px_h) = (image.width(), image.height());
        let layout = Layout::new(self.page_size, px_w as f32, px_h as f32);

        let page_id = self.alloc();
        let content_id = self.alloc();
        let image_id = self.alloc();
        let mask_id = image.color().has_alpha().then(|| self.alloc());

        // Page object.
        let mut page = self.pdf.page(page_id);
        page.media_box(Rect::new(0.0, 0.0, layout.page_w, layout.page_h));
        page.parent(self.page_tree);
        page.contents(content_id);
        page.resources().x_objects().pair(IMAGE_NAME, image_id);
        page.finish();

        // Image data (+ alpha mask).
        let (filter, data) = match mask_id {
            None => (Filter::DctDecode, self.encode_jpeg(image)?),
            Some(_) => (Filter::FlateDecode, deflate(image.to_rgb8().as_raw())),
        };
        let mut xobject = self.pdf.image_xobject(image_id, &data);
        xobject.filter(filter);
        xobject.width(px_w as i32);
        xobject.height(px_h as i32);
        xobject.color_space().device_rgb();
        xobject.bits_per_component(8);
        if let Some(mask_id) = mask_id {
            xobject.s_mask(mask_id);
        }
        xobject.finish();

        if let Some(mask_id) = mask_id {
            let alpha: Vec<u8> = image.to_rgba8().pixels().map(|p| p.0[3]).collect();
            let alpha = deflate(&alpha);
            let mut mask = self.pdf.image_xobject(mask_id, &alpha);
            mask.filter(Filter::FlateDecode);
            mask.width(px_w as i32);
            mask.height(px_h as i32);
            mask.color_space().device_gray();
            mask.bits_per_component(8);
            mask.finish();
        }

        // Draw the image: XObjects are 1×1 units, so scale and move them with
        // the transformation matrix [sx 0 0 sy tx ty]. The origin is bottom-left.
        let mut content = Content::new();
        content.save_state();
        content.transform([layout.draw_w, 0.0, 0.0, layout.draw_h, layout.x, layout.y]);
        content.x_object(IMAGE_NAME);
        content.restore_state();
        self.pdf.stream(content_id, &content.finish());

        self.pages.push(page_id);
        Ok(())
    }

    /// Writes the page tree and returns the finished PDF file.
    pub(crate) fn finish(mut self) -> Vec<u8> {
        let info_id = self.alloc();
        self.pdf.catalog(self.catalog).pages(self.page_tree);
        self.pdf
            .pages(self.page_tree)
            .kids(self.pages.iter().copied())
            .count(self.pages.len() as i32);
        self.pdf
            .document_info(info_id)
            .producer(TextStr("ConvertApp"));
        self.pdf.finish()
    }

    fn alloc(&mut self) -> Ref {
        let id = Ref::new(self.next_ref);
        self.next_ref += 1;
        id
    }

    fn encode_jpeg(&self, image: &DynamicImage) -> Result<Vec<u8>, ImageError> {
        let mut data = Vec::new();
        DynamicImage::ImageRgb8(image.to_rgb8())
            .write_with_encoder(JpegEncoder::new_with_quality(&mut data, self.quality))?;
        Ok(data)
    }
}

/// Where an image goes on its page, in PDF points.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Layout {
    page_w: f32,
    page_h: f32,
    draw_w: f32,
    draw_h: f32,
    x: f32,
    y: f32,
}

impl Layout {
    /// Computes the page size and image placement for an image of
    /// `px_w`×`px_h` pixels.
    fn new(page_size: PdfPageSize, px_w: f32, px_h: f32) -> Self {
        let (natural_w, natural_h) = (px_w * PT_PER_PX, px_h * PT_PER_PX);
        match page_size {
            PdfPageSize::Image => Self {
                page_w: natural_w,
                page_h: natural_h,
                draw_w: natural_w,
                draw_h: natural_h,
                x: 0.0,
                y: 0.0,
            },
            PdfPageSize::A4 => {
                // Landscape for wide images.
                let (page_w, page_h) = if px_w > px_h { (A4.1, A4.0) } else { A4 };
                let scale = ((page_w - 2.0 * MARGIN) / natural_w)
                    .min((page_h - 2.0 * MARGIN) / natural_h)
                    .min(1.0); // shrink to fit, never enlarge
                let (draw_w, draw_h) = (natural_w * scale, natural_h * scale);
                Self {
                    page_w,
                    page_h,
                    draw_w,
                    draw_h,
                    x: (page_w - draw_w) / 2.0,
                    y: (page_h - draw_h) / 2.0,
                }
            }
        }
    }
}

fn deflate(data: &[u8]) -> Vec<u8> {
    compress_to_vec_zlib(data, CompressionLevel::DefaultLevel as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_page_matches_image_size() {
        let layout = Layout::new(PdfPageSize::Image, 960.0, 480.0);
        assert_eq!((layout.page_w, layout.page_h), (720.0, 360.0));
        assert_eq!((layout.x, layout.y), (0.0, 0.0));
    }

    #[test]
    fn a4_shrinks_large_images_and_centres_them() {
        let layout = Layout::new(PdfPageSize::A4, 4000.0, 3000.0);
        assert_eq!(
            (layout.page_w, layout.page_h),
            (A4.1, A4.0),
            "landscape for wide images"
        );
        assert!(layout.draw_w <= layout.page_w - 2.0 * MARGIN + 0.01);
        assert!(layout.draw_h <= layout.page_h - 2.0 * MARGIN + 0.01);
        assert!((layout.x * 2.0 + layout.draw_w - layout.page_w).abs() < 0.01);
    }

    #[test]
    fn a4_does_not_enlarge_small_images() {
        let layout = Layout::new(PdfPageSize::A4, 100.0, 200.0);
        assert_eq!((layout.draw_w, layout.draw_h), (75.0, 150.0));
        assert_eq!(
            (layout.page_w, layout.page_h),
            A4,
            "portrait for tall images"
        );
    }

    #[test]
    fn builds_a_multi_page_pdf() {
        let mut pdf = PdfBuilder::new(&ImageOptions::default());
        pdf.add_image(&DynamicImage::new_rgb8(10, 10)).unwrap();
        pdf.add_image(&DynamicImage::new_rgba8(10, 10)).unwrap();
        let bytes = pdf.finish();
        let text = String::from_utf8_lossy(&bytes);
        assert!(bytes.starts_with(b"%PDF-"));
        assert!(text.contains("/Count 2"));
        assert!(text.contains("/DCTDecode"), "opaque image embedded as JPEG");
        assert!(
            text.contains("/SMask"),
            "transparent image has an alpha mask"
        );
    }
}
