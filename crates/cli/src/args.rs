//! Command-line argument definitions (parsed by `clap`).

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use convertapp_core::{Format, ImageOptions};

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
    /// Target format, as an extension or name: jpg, png, bmp, gif, tiff, ico…
    #[arg(short, long, value_name = "FORMAT")]
    pub to: Format,

    /// Output folder (created if missing). Default: next to each input file.
    #[arg(short, long, value_name = "DIR")]
    pub out: Option<PathBuf>,

    /// JPEG quality, 1–100.
    #[arg(
        short,
        long,
        default_value_t = ImageOptions::DEFAULT_JPEG_QUALITY,
        value_parser = clap::value_parser!(u8).range(1..=100)
    )]
    pub quality: u8,

    /// Files to convert.
    #[arg(required = true)]
    pub files: Vec<PathBuf>,
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
