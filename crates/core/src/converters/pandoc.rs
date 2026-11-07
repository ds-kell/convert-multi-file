//! Document conversions delegated to [Pandoc](https://pandoc.org):
//! Word ↔ Markdown/HTML, and Markdown → PDF (typeset by Typst).
//!
//! Pandoc is an external program (see [`crate::tools`]). When it is missing,
//! these converters report [`Availability::Unavailable`] with an install
//! hint, and the UI shows the formats as locked instead of failing.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::tools::{self, PANDOC, TYPST};
use crate::{Availability, ConvertError, Converter, Format, Progress, Request, Result};

/// Upper bound for one Pandoc run. Large documents take a few seconds; this
/// only protects against a hung process.
const TIMEOUT: Duration = Duration::from_secs(180);

/// Word ↔ Markdown / HTML through Pandoc.
///
/// - Markdown is read and written as GitHub-Flavored Markdown, matching the
///   built-in Markdown converter.
/// - Word → Markdown saves embedded pictures into a `<name>_files` folder
///   next to the output and links to them.
/// - Word → HTML produces a single self-contained page (pictures embedded).
/// - Generated Word files can take their styles from a reference document
///   ([`crate::DocumentOptions::reference_docx`]).
#[derive(Debug, Default, Clone, Copy)]
pub struct PandocConverter;

impl Converter for PandocConverter {
    fn id(&self) -> &'static str {
        "pandoc"
    }

    fn inputs(&self) -> &[Format] {
        &[Format::Markdown, Format::Html, Format::Docx]
    }

    fn outputs(&self, input: Format) -> Vec<Format> {
        match input {
            Format::Markdown | Format::Html => vec![Format::Docx],
            Format::Docx => vec![Format::Markdown, Format::Html],
            _ => Vec::new(),
        }
    }

    fn availability(&self) -> Availability {
        PANDOC.availability()
    }

    fn convert(&self, request: &Request<'_>, progress: &dyn Progress) -> Result<()> {
        let pandoc = PANDOC.require()?;
        let args = document_args(request)?;
        progress.set(0.2);
        tools::run(&PANDOC, &pandoc, args, output_dir(request.output), TIMEOUT)
    }
}

/// Markdown → PDF with Pandoc and Typst (no LaTeX installation needed).
///
/// Runs in two steps instead of Pandoc's `--pdf-engine=typst`: Pandoc turns
/// image references into absolute `C:\…` paths for the engine, which Typst
/// rejects on Windows. So Pandoc only writes Typst source next to the
/// Markdown file (keeping relative image paths), and Typst compiles it with
/// the drive as project root.
#[derive(Debug, Default, Clone, Copy)]
pub struct MarkdownToPdfConverter;

impl Converter for MarkdownToPdfConverter {
    fn id(&self) -> &'static str {
        "pandoc-pdf"
    }

    fn inputs(&self) -> &[Format] {
        &[Format::Markdown]
    }

    fn outputs(&self, input: Format) -> Vec<Format> {
        match input {
            Format::Markdown => vec![Format::Pdf],
            _ => Vec::new(),
        }
    }

    fn availability(&self) -> Availability {
        // Both programs are needed; report the first one missing.
        match PANDOC.availability() {
            Availability::Available => TYPST.availability(),
            missing => missing,
        }
    }

    fn convert(&self, request: &Request<'_>, progress: &dyn Progress) -> Result<()> {
        let pandoc = PANDOC.require()?;
        let typst = TYPST.require()?;
        let input = absolute(request.input)?;
        let input_dir = input.parent().unwrap_or(Path::new("."));

        // 1. Markdown → Typst source, as a hidden temp file beside the input
        //    so relative image paths resolve from the same place.
        let source = tempfile::Builder::new()
            .prefix(".convertapp-")
            .suffix(".typ")
            .tempfile_in(input_dir)
            .map_err(|e| ConvertError::io(input_dir, e))?
            .into_temp_path();
        tools::run(
            &PANDOC,
            &pandoc,
            typst_source_args(&input, &source),
            input_dir,
            TIMEOUT,
        )?;
        progress.set(0.5);

        // 2. Typst source → PDF. The root must contain every referenced file;
        //    the input's drive (or file-system root) always does.
        let root = input.ancestors().last().unwrap_or(input_dir);
        let compile: [OsString; 5] = [
            "compile".into(),
            source.to_path_buf().into(),
            request.output.into(),
            "--root".into(),
            root.into(),
        ];
        tools::run(&TYPST, &typst, compile, input_dir, TIMEOUT)
    }
}

/// Pandoc's name for one of our formats.
fn pandoc_format(format: Format) -> &'static str {
    match format {
        Format::Markdown => "gfm",
        Format::Html => "html",
        Format::Docx => "docx",
        other => unreachable!("{other} is not handled by Pandoc converters"),
    }
}

/// Arguments shared by every run: input, formats, output, resource path.
fn base_args(request: &Request<'_>, input: &Path) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec![
        input.into(),
        "--from".into(),
        pandoc_format(request.input_format).into(),
        "--output".into(),
        request.output.into(),
    ];
    // Relative image paths in the source are resolved next to the source.
    if let Some(dir) = input.parent() {
        args.push(join("--resource-path=", dir));
    }
    args
}

/// Arguments for [`PandocConverter`].
fn document_args(request: &Request<'_>) -> Result<Vec<OsString>> {
    let input = absolute(request.input)?;
    let mut args = base_args(request, &input);
    args.push("--to".into());
    args.push(pandoc_format(request.target).into());

    match request.target {
        Format::Docx => {
            if let Some(reference) = &request.options.document.reference_docx {
                args.push(join("--reference-doc=", &absolute(reference)?));
            }
        }
        Format::Markdown => {
            // Pandoc runs in the output folder, so this relative path is
            // also what the Markdown links will contain.
            let media = media_dir(&input, output_dir(request.output));
            args.push(join("--extract-media=", Path::new(&media)));
            args.push("--wrap=none".into());
        }
        Format::Html => {
            args.push("--standalone".into());
            args.push("--embed-resources".into());
            args.push("--metadata".into());
            args.push(format!("pagetitle={}", file_stem(&input)).into());
        }
        _ => {}
    }
    Ok(args)
}

/// Pandoc arguments for step 1 of [`MarkdownToPdfConverter`]: Markdown →
/// standalone Typst source written to `source`.
fn typst_source_args(input: &Path, source: &Path) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec![
        input.into(),
        "--from".into(),
        pandoc_format(Format::Markdown).into(),
        "--to".into(),
        "typst".into(),
        "--standalone".into(),
        "--output".into(),
        source.into(),
        "--variable=papersize=a4".into(),
    ];
    // Use a system font with full Vietnamese/Latin coverage. Only one value
    // per variable: Pandoc concatenates repeated values into one bogus name.
    // Elsewhere Typst's bundled fonts are used.
    if let Some((main, code)) = PDF_FONTS {
        args.push(format!("--variable=mainfont={main}").into());
        args.push(format!("--variable=codefont={code}").into());
    }
    args
}

/// `(body font, code font)` for PDFs, chosen per platform.
const PDF_FONTS: Option<(&str, &str)> = if cfg!(windows) {
    Some(("Segoe UI", "Consolas"))
} else if cfg!(target_os = "macos") {
    Some(("Helvetica Neue", "Menlo"))
} else {
    None
};

/// Folder the output file is written in (Pandoc's working directory).
fn output_dir(output: &Path) -> &Path {
    output.parent().unwrap_or(Path::new("."))
}

/// A free `<input stem>_files` folder name inside `dir` for extracted media.
fn media_dir(input: &Path, dir: &Path) -> String {
    let stem = file_stem(input);
    let base = format!("{stem}_files");
    if !dir.join(&base).exists() {
        return base;
    }
    (1u32..)
        .map(|n| format!("{stem}_files ({n})"))
        .find(|name| !dir.join(name).exists())
        .expect("an unused folder name always exists")
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document".to_owned())
}

/// `--flag=` followed by a path, as one argument.
fn join(flag: &str, path: &Path) -> OsString {
    let mut arg = OsString::from(flag);
    arg.push(path);
    arg
}

/// Pandoc runs in the output folder, so relative input paths must be made
/// absolute first.
fn absolute(path: &Path) -> Result<PathBuf> {
    std::path::absolute(path).map_err(|e| ConvertError::io(path, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConvertOptions, DocumentOptions};

    fn request<'a>(
        input: &'a Path,
        from: Format,
        output: &'a Path,
        to: Format,
        options: &'a ConvertOptions,
    ) -> Request<'a> {
        Request {
            input,
            input_format: from,
            output,
            target: to,
            options,
        }
    }

    fn strings(args: &[OsString]) -> Vec<String> {
        args.iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn markdown_to_docx_uses_gfm_and_reference_doc() {
        let options = ConvertOptions {
            document: DocumentOptions {
                reference_docx: Some(PathBuf::from(r"C:\styles\company.docx")),
            },
            ..ConvertOptions::default()
        };
        let args = strings(
            &document_args(&request(
                Path::new(r"C:\docs\notes.md"),
                Format::Markdown,
                Path::new(r"C:\docs\.tmp.docx"),
                Format::Docx,
                &options,
            ))
            .unwrap(),
        );
        assert!(args.windows(2).any(|w| w == ["--from", "gfm"]));
        assert!(args.windows(2).any(|w| w == ["--to", "docx"]));
        assert!(args.contains(&r"--resource-path=C:\docs".to_owned()));
        assert!(args.contains(&r"--reference-doc=C:\styles\company.docx".to_owned()));
    }

    #[test]
    fn docx_to_html_is_self_contained() {
        let options = ConvertOptions::default();
        let args = strings(
            &document_args(&request(
                Path::new(r"C:\docs\report.docx"),
                Format::Docx,
                Path::new(r"C:\out\.tmp.html"),
                Format::Html,
                &options,
            ))
            .unwrap(),
        );
        assert!(args.contains(&"--embed-resources".to_owned()));
        assert!(args.contains(&"pagetitle=report".to_owned()));
    }

    #[test]
    fn media_dir_avoids_existing_folders() {
        let dir = tempfile::tempdir().unwrap();
        let input = Path::new("report.docx");
        assert_eq!(media_dir(input, dir.path()), "report_files");
        std::fs::create_dir(dir.path().join("report_files")).unwrap();
        assert_eq!(media_dir(input, dir.path()), "report_files (1)");
    }

    #[test]
    fn pairs_are_declared() {
        assert!(PandocConverter.supports(Format::Markdown, Format::Docx));
        assert!(PandocConverter.supports(Format::Docx, Format::Html));
        assert!(!PandocConverter.supports(Format::Markdown, Format::Html));
        assert!(MarkdownToPdfConverter.supports(Format::Markdown, Format::Pdf));
    }
}
