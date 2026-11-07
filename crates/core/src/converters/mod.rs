//! Built-in [`Converter`](crate::Converter) implementations, one module per
//! format family.

mod html;
pub(crate) mod image;
mod markdown;
mod office;
mod pandoc;
pub(crate) mod svg;
mod web;

pub use self::html::HtmlToMarkdownConverter;
pub use self::image::ImageConverter;
pub use self::markdown::MarkdownConverter;
pub use self::office::libreoffice::LibreOfficeConverter;
pub use self::office::msoffice::MsOfficeConverter;
pub use self::pandoc::{MarkdownToPdfConverter, PandocConverter};
pub use self::svg::SvgConverter;
pub use self::web::WebConverter;
