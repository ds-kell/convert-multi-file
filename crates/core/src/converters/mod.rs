//! Built-in [`Converter`](crate::Converter) implementations, one module per
//! format family.

mod html;
pub(crate) mod image;
mod markdown;
mod pandoc;
pub(crate) mod svg;

pub use self::html::HtmlToMarkdownConverter;
pub use self::image::ImageConverter;
pub use self::markdown::MarkdownConverter;
pub use self::pandoc::{MarkdownToPdfConverter, PandocConverter};
pub use self::svg::SvgConverter;
