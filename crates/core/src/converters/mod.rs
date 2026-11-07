//! Built-in [`Converter`](crate::Converter) implementations, one module per
//! format family.

mod html;
mod image;
mod markdown;

pub use self::html::HtmlToMarkdownConverter;
pub use self::image::ImageConverter;
pub use self::markdown::MarkdownConverter;
