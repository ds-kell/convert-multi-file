//! `convertapp-cli` – command-line front end for `convertapp-core`.
//!
//! ```text
//! convertapp-cli formats photo.png
//! convertapp-cli convert --to jpg photo.png scan.tiff --out converted/
//! ```
//!
//! Exit codes: `0` every file converted, `1` at least one file failed,
//! `2` invalid command-line arguments.

mod args;

use std::path::Path;
use std::process::ExitCode;

use clap::Parser;
use convertapp_core::{
    ConvertOptions, Format, HtmlOptions, ImageOptions, Job, NoProgress, Registry, convert,
    error_chain,
};

use crate::args::{Cli, Command, ConvertArgs};

fn main() -> ExitCode {
    let cli = Cli::parse();
    init_logging(cli.verbose);

    let registry = Registry::with_defaults();
    let all_ok = match &cli.command {
        Command::Formats { files } => list_formats(&registry, files),
        Command::Convert(args) => convert_files(&registry, args),
    };

    if all_ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Sends `tracing` logs to stderr. `-v` shows info, `-vv` debug, `-vvv` trace.
/// `RUST_LOG` overrides the flag when set.
fn init_logging(verbosity: u8) {
    let level = match verbosity {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .init();
}

/// `formats` sub-command: prints the possible targets for each file.
/// Returns `false` if any file could not be inspected.
fn list_formats(registry: &Registry, files: &[impl AsRef<Path>]) -> bool {
    let mut all_ok = true;

    for file in files {
        let file = file.as_ref();
        match Format::detect(file) {
            Ok(Some(format)) => {
                let targets: Vec<&str> = registry
                    .targets_for(format)
                    .into_iter()
                    .map(Format::extension)
                    .collect();
                let targets = if targets.is_empty() {
                    "(none)".to_owned()
                } else {
                    targets.join(", ")
                };
                println!("{} ({format}) -> {targets}", file.display());
            }
            Ok(None) => {
                eprintln!("error: {}: unknown format", file.display());
                all_ok = false;
            }
            Err(err) => {
                eprintln!("error: {}: {}", file.display(), error_chain(&err));
                all_ok = false;
            }
        }
    }
    all_ok
}

/// `convert` sub-command: converts every file, continuing after failures.
/// Returns `false` if any file failed.
fn convert_files(registry: &Registry, args: &ConvertArgs) -> bool {
    let options = ConvertOptions {
        image: ImageOptions {
            jpeg_quality: args.quality,
            ..ImageOptions::default()
        },
        html: HtmlOptions {
            theme: args.theme.into(),
        },
    };

    let mut failed = 0usize;
    for file in &args.files {
        let mut job = Job::new(file, args.to).with_options(options.clone());
        if let Some(dir) = &args.out {
            job = job.with_output_dir(dir);
        }

        match convert(registry, &job, &NoProgress) {
            Ok(output) => println!("[ok]    {} -> {}", file.display(), output.display()),
            Err(err) => {
                eprintln!("[error] {}: {}", file.display(), error_chain(&err));
                failed += 1;
            }
        }
    }

    let total = args.files.len();
    if total > 1 {
        println!("{} converted, {failed} failed", total - failed);
    }
    failed == 0
}
