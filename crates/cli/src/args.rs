//! Command-line argument definitions (parsed by `clap`).

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use convertapp_core::{Format, HtmlTheme, ImageOptions, PdfPageSize, Resize};

/// Convert files between formats (images, documents, …).
#[derive(Debug, Parser)]
#[command(name = "convertapp-cli", version, about)]
pub struct Cli {
    /// More log output on stderr (-v info, -vv debug, -vvv trace).
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Command,
}

/// Available sub-commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Show which formats each file can be converted to.
    Formats {
        /// Files to inspect.
        #[arg(required = true)]
        files: Vec<PathBuf>,
    },

    /// Convert files to another format.
    Convert(ConvertArgs),
}

/// Arguments of the `convert` sub-command.
#[derive(Debug, Args)]
pub struct ConvertArgs {
    /// Target format, as an extension or name: jpg, png, webp, gif, ico, pdf, md, html…
    #[arg(short, long, value_name = "FORMAT")]
    pub to: Format,

    /// Output folder (created if missing). Default: next to each input file.
    #[arg(short, long, value_name = "DIR")]
    pub out: Option<PathBuf>,

    /// Quality of lossy output (JPEG, WebP, photos in PDF), 1–100.
    #[arg(
        short,
        long,
        default_value_t = ImageOptions::DEFAULT_QUALITY,
        value_parser = clap::value_parser!(u8).range(1..=100)
    )]
    pub quality: u8,

    /// Write WebP without quality loss (ignores --quality).
    #[arg(long)]
    pub lossless: bool,

    /// Shrink images to at most this width in pixels (keeps aspect ratio).
    #[arg(long, value_name = "PX", conflicts_with = "scale")]
    pub max_width: Option<u32>,

    /// Shrink images to at most this height in pixels (keeps aspect ratio).
    #[arg(long, value_name = "PX", conflicts_with = "scale")]
    pub max_height: Option<u32>,

    /// Scale images by this percentage (e.g. 50 = half size).
    #[arg(long, value_name = "PERCENT", value_parser = clap::value_parser!(u32).range(1..=1000))]
    pub scale: Option<u32>,

    /// Page size for images converted to PDF.
    #[arg(long, value_enum, default_value_t = Page::A4)]
    pub page: Page,

    /// With --to pdf: put all images into a single PDF (one page each).
    #[arg(long)]
    pub merge: bool,

    /// Colour theme of HTML generated from Markdown.
    #[arg(long, value_enum, default_value_t = Theme::Auto)]
    pub theme: Theme,

    /// Files to convert.
    #[arg(required = true)]
    pub files: Vec<PathBuf>,
}

impl ConvertArgs {
    /// The resize setting described by `--max-width`, `--max-height` and `--scale`.
    pub fn resize(&self) -> Resize {
        match (self.scale, self.max_width, self.max_height) {
            (Some(percent), _, _) => Resize::Percent(percent),
            (None, None, None) => Resize::Original,
            (None, max_width, max_height) => Resize::Fit {
                max_width,
                max_height,
            },
        }
    }
}

/// CLI spelling of [`PdfPageSize`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Page {
    /// A4 with margins, landscape for wide images.
    A4,
    /// Page exactly as large as the image.
    Image,
}

impl From<Page> for PdfPageSize {
    fn from(page: Page) -> Self {
        match page {
            Page::A4 => PdfPageSize::A4,
            Page::Image => PdfPageSize::Image,
        }
    }
}

/// CLI spelling of [`HtmlTheme`] (clap needs its own `ValueEnum` type).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Theme {
    /// Follow the reader's light/dark system setting.
    Auto,
    /// Always light.
    Light,
    /// Always dark.
    Dark,
    /// No stylesheet.
    None,
}

impl From<Theme> for HtmlTheme {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Auto => HtmlTheme::Auto,
            Theme::Light => HtmlTheme::Light,
            Theme::Dark => HtmlTheme::Dark,
            Theme::None => HtmlTheme::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }
}
