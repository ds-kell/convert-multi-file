//! LibreOffice engine: `soffice --headless --convert-to`.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::{allowed, outputs_for};
use crate::tools::{self, LIBREOFFICE};
use crate::{
    Availability, ConvertError, ConvertOptions, Converter, Format, OfficeEngine, Progress, Request,
    Result,
};

/// Generous: the first start of a fresh profile alone takes a few seconds.
const TIMEOUT: Duration = Duration::from_secs(300);

/// Converts office documents with LibreOffice running headless.
///
/// - Every conversion uses its own throw-away LibreOffice profile, so
///   several conversions can run at once and nothing leaks between them.
/// - CSV is read and written as UTF-8 with commas; exporting a workbook to
///   CSV writes its first sheet.
/// - No plain-text output: LibreOffice 26.8 hangs in headless mode when
///   exporting a document containing a table to text. Word or Pandoc
///   provide that conversion instead.
#[derive(Debug, Default, Clone, Copy)]
pub struct LibreOfficeConverter;

impl Converter for LibreOfficeConverter {
    fn id(&self) -> &'static str {
        "libreoffice"
    }

    fn inputs(&self) -> &[Format] {
        INPUTS
    }

    fn outputs(&self, input: Format) -> Vec<Format> {
        if input == Format::Pdf {
            // Imported into Writer as positioned text frames: editable, but
            // less tidy than Word's result, hence registered after Word.
            return vec![Format::Docx, Format::Odt];
        }
        // See the type documentation for why text is excluded.
        outputs_for(input)
            .into_iter()
            .filter(|&f| f != Format::Txt)
            .collect()
    }

    fn availability(&self) -> Availability {
        LIBREOFFICE.availability()
    }

    fn is_enabled(&self, options: &ConvertOptions) -> bool {
        allowed(options, OfficeEngine::LibreOffice)
    }

    fn convert(&self, request: &Request<'_>, progress: &dyn Progress) -> Result<()> {
        let soffice = console_launcher(&LIBREOFFICE.require()?);
        let filter = export_filter(request.target).ok_or(ConvertError::Unsupported {
            from: request.input_format,
            to: request.target,
        })?;
        let input = absolute(request.input)?;
        let output = absolute(request.output)?;
        let dest_dir = output.parent().unwrap_or(Path::new(".")).to_path_buf();

        // LibreOffice names the result after the input; let it write into a
        // private folder on the destination drive, then move it into place.
        let out_dir = tempfile::Builder::new()
            .prefix(".convertapp-lo-")
            .tempdir_in(&dest_dir)
            .map_err(|e| ConvertError::io(&dest_dir, e))?;
        let profile = tempfile::Builder::new()
            .prefix("convertapp-lo-profile-")
            .tempdir()
            .map_err(|e| ConvertError::io(std::env::temp_dir(), e))?;
        let profile_url = url::Url::from_directory_path(profile.path())
            .map_err(|()| ConvertError::decode(profile.path(), "invalid profile path"))?;

        let mut args: Vec<OsString> = vec![
            "--headless".into(),
            "--norestore".into(),
            "--nolockcheck".into(),
            "--nodefault".into(),
            format!("-env:UserInstallation={profile_url}").into(),
        ];
        match request.input_format {
            // Comma separated, double quotes, UTF-8 (76), start at line 1.
            Format::Csv => args.push("--infilter=CSV:44,34,76,1".into()),
            // Without this, PDFs open in Draw, which cannot save Word files.
            Format::Pdf => args.push("--infilter=writer_pdf_import".into()),
            _ => {}
        }
        args.extend([
            "--convert-to".into(),
            filter.into(),
            "--outdir".into(),
            out_dir.path().into(),
            input.clone().into(),
        ]);

        progress.set(0.1);
        tools::run(&LIBREOFFICE, &soffice, args, &dest_dir, TIMEOUT)?;
        progress.set(0.9);

        // LibreOffice exits with 0 even when it could not load the file, so
        // the produced file is the real success signal.
        let stem = input.file_stem().unwrap_or_default();
        let produced = out_dir
            .path()
            .join(stem)
            .with_extension(request.target.extension());
        if !produced.is_file() {
            return Err(ConvertError::External {
                tool: LIBREOFFICE.name,
                message: format!(
                    "could not convert '{}' (damaged or password-protected?)",
                    input.display()
                ),
            });
        }
        fs::rename(&produced, &output).map_err(|e| ConvertError::io(&output, e))
    }
}

/// Office formats plus PDF (opened in Writer for PDF → Word).
const INPUTS: &[Format] = &[
    Format::Docx,
    Format::Doc,
    Format::Odt,
    Format::Rtf,
    Format::Xlsx,
    Format::Xls,
    Format::Ods,
    Format::Csv,
    Format::Pptx,
    Format::Ppt,
    Format::Odp,
    Format::Pdf,
];

/// On Windows, `soffice.exe` is a GUI launcher that returns before the
/// conversion finishes; `soffice.com` next to it is the console variant that
/// waits and reports the exit code.
fn console_launcher(soffice: &Path) -> PathBuf {
    let console = soffice.with_extension("com");
    if cfg!(windows) && console.is_file() {
        console
    } else {
        soffice.to_path_buf()
    }
}

/// `--convert-to` argument for `target`: extension, optionally followed by
/// the export filter name and its options.
fn export_filter(target: Format) -> Option<&'static str> {
    Some(match target {
        Format::Pdf => "pdf",
        Format::Docx => "docx:MS Word 2007 XML",
        Format::Odt => "odt",
        Format::Rtf => "rtf",
        Format::Xlsx => "xlsx:Calc MS Excel 2007 XML",
        Format::Ods => "ods",
        Format::Csv => "csv:Text - txt - csv (StarCalc):44,34,76,1",
        Format::Pptx => "pptx:Impress MS PowerPoint 2007 XML",
        Format::Odp => "odp",
        _ => return None,
    })
}

fn absolute(path: &Path) -> Result<PathBuf> {
    std::path::absolute(path).map_err(|e| ConvertError::io(path, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_declared_output_has_a_filter() {
        for &input in INPUTS {
            for target in LibreOfficeConverter.outputs(input) {
                assert!(export_filter(target).is_some(), "{input} -> {target}");
            }
        }
    }

    #[test]
    fn respects_engine_choice() {
        let mut options = ConvertOptions::default();
        options.office.engine = OfficeEngine::MicrosoftOffice;
        assert!(!LibreOfficeConverter.is_enabled(&options));
        options.office.engine = OfficeEngine::LibreOffice;
        assert!(LibreOfficeConverter.is_enabled(&options));
    }
}
