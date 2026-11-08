//! Microsoft Office engine: Word, Excel and PowerPoint driven through COM.
//!
//! COM automation is done by a small PowerShell script
//! (`assets/office.ps1`) rather than from Rust: PowerShell's late-bound COM
//! support keeps the Office specifics readable, and a crash or hang inside
//! Office cannot take ConvertApp down with it.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use super::{Kind, allowed, outputs_for};
use crate::tools::{self, Tool};
use crate::{
    Availability, ConvertError, ConvertOptions, Converter, Format, OfficeEngine, Progress, Request,
    Requirement, Result,
};

/// The automation script, embedded in the binary.
const SCRIPT: &str = include_str!("../../../assets/office.ps1");

/// Large workbooks and presentations can take a while to export.
const TIMEOUT: Duration = Duration::from_secs(300);

/// PowerShell, presented as "Microsoft Office" in error messages because
/// that is what actually fails.
const POWERSHELL: Tool = Tool {
    name: "Microsoft Office",
    program: "powershell",
    env_override: "CONVERTAPP_POWERSHELL",
    known_dirs: &[("SystemRoot", r"System32\WindowsPowerShell\v1.0")],
    app_path: None,
    homepage: "https://learn.microsoft.com/powershell/",
    install_command: None,
};

/// Shown when Word/Excel/PowerPoint is missing. Paid, so the UI lists the
/// free LibreOffice alternative first.
const MICROSOFT_OFFICE: Requirement = Requirement {
    name: "Microsoft Office",
    url: "https://www.microsoft.com/microsoft-365",
    install_command: None,
    free: false,
};

/// One Office conversion at a time: PowerPoint is a single shared instance,
/// so parallel jobs would close each other's presentations.
static OFFICE_LOCK: Mutex<()> = Mutex::new(());

/// The Office application that handles a kind of document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum App {
    Word,
    Excel,
    PowerPoint,
}

impl App {
    fn kind(self) -> Kind {
        match self {
            App::Word => Kind::Text,
            App::Excel => Kind::Spreadsheet,
            App::PowerPoint => Kind::Presentation,
        }
    }

    /// COM class registered by the Office installer.
    fn prog_id(self) -> &'static str {
        match self {
            App::Word => "Word.Application",
            App::Excel => "Excel.Application",
            App::PowerPoint => "PowerPoint.Application",
        }
    }

    /// `-App` value understood by the script.
    fn script_name(self) -> &'static str {
        match self {
            App::Word => "word",
            App::Excel => "excel",
            App::PowerPoint => "powerpoint",
        }
    }

    /// `-Format` value for `target`: "pdf", "txt" or the SaveAs file-format
    /// constant from the Office object model.
    fn format_code(self, target: Format) -> Option<&'static str> {
        Some(match (self, target) {
            (_, Format::Pdf) => "pdf",
            (App::Word, Format::Txt) => "txt",
            (App::Word, Format::Docx) => "16", // wdFormatDocumentDefault
            (App::Word, Format::Odt) => "23",  // wdFormatOpenDocumentText
            (App::Word, Format::Rtf) => "6",   // wdFormatRTF
            (App::Excel, Format::Xlsx) => "51", // xlOpenXMLWorkbook
            (App::Excel, Format::Ods) => "60", // xlOpenDocumentSpreadsheet
            (App::Excel, Format::Csv) => "62", // xlCSVUTF8 (active sheet)
            (App::PowerPoint, Format::Pptx) => "24", // ppSaveAsOpenXMLPresentation
            (App::PowerPoint, Format::Odp) => "35", // ppSaveAsOpenDocumentPresentation
            _ => return None,
        })
    }
}

/// Converts office documents with the installed Microsoft Office.
///
/// One converter per application (Word, Excel, PowerPoint), because each can
/// be installed – or missing – independently.
#[derive(Debug, Clone, Copy)]
pub struct MsOfficeConverter {
    app: App,
}

impl MsOfficeConverter {
    /// Word: docx/doc/odt/rtf → pdf, docx, odt, rtf, txt.
    pub fn word() -> Self {
        Self { app: App::Word }
    }

    /// Excel: xlsx/xls/ods/csv → pdf, xlsx, ods, csv.
    pub fn excel() -> Self {
        Self { app: App::Excel }
    }

    /// PowerPoint: pptx/ppt/odp → pdf, pptx, odp.
    pub fn powerpoint() -> Self {
        Self {
            app: App::PowerPoint,
        }
    }
}

impl Converter for MsOfficeConverter {
    fn id(&self) -> &'static str {
        match self.app {
            App::Word => "ms-word",
            App::Excel => "ms-excel",
            App::PowerPoint => "ms-powerpoint",
        }
    }

    fn inputs(&self) -> &[Format] {
        self.app.kind().inputs()
    }

    fn outputs(&self, input: Format) -> Vec<Format> {
        if !self.inputs().contains(&input) {
            return Vec::new();
        }
        outputs_for(input)
    }

    fn availability(&self) -> Availability {
        if is_installed(self.app.prog_id()) && POWERSHELL.locate().is_some() {
            Availability::Available
        } else {
            Availability::missing(vec![MICROSOFT_OFFICE])
        }
    }

    fn is_enabled(&self, options: &ConvertOptions) -> bool {
        allowed(options, OfficeEngine::MicrosoftOffice)
    }

    fn convert(&self, request: &Request<'_>, progress: &dyn Progress) -> Result<()> {
        let format = self
            .app
            .format_code(request.target)
            .ok_or(ConvertError::Unsupported {
                from: request.input_format,
                to: request.target,
            })?;
        let powershell = POWERSHELL.require()?;
        let source = absolute(request.input)?;
        let target = absolute(request.output)?;
        let work_dir = target.parent().unwrap_or(Path::new(".")).to_path_buf();

        // The script and the PID file live in a private temp folder.
        let scratch = tempfile::Builder::new()
            .prefix("convertapp-office-")
            .tempdir()
            .map_err(|e| ConvertError::io(std::env::temp_dir(), e))?;
        let script = scratch.path().join("office.ps1");
        // UTF-8 with BOM so Windows PowerShell 5 reads non-ASCII text correctly.
        fs::write(&script, format!("\u{FEFF}{SCRIPT}"))
            .map_err(|e| ConvertError::io(&script, e))?;
        let pid_file = scratch.path().join("office.pid");

        let args: Vec<std::ffi::OsString> = vec![
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-ExecutionPolicy".into(),
            "Bypass".into(),
            "-File".into(),
            script.into(),
            "-App".into(),
            self.app.script_name().into(),
            "-Source".into(),
            source.into(),
            "-Target".into(),
            target.into(),
            "-Format".into(),
            format.into(),
            "-PidFile".into(),
            pid_file.clone().into(),
        ];

        let _guard = OFFICE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        progress.set(0.1);
        let result = tools::run(&POWERSHELL, &powershell, args, &work_dir, TIMEOUT);
        if result.is_err() {
            // The script normally closes Office itself; after a timeout or a
            // crash, make sure no invisible Office process is left behind.
            kill_started_office(&pid_file);
        }
        result
    }
}

/// `true` if the COM class `prog_id` is registered (Office app installed).
#[cfg(windows)]
fn is_installed(prog_id: &str) -> bool {
    use winreg::RegKey;
    use winreg::enums::HKEY_CLASSES_ROOT;
    RegKey::predef(HKEY_CLASSES_ROOT)
        .open_subkey(format!(r"{prog_id}\CLSID"))
        .is_ok()
}

#[cfg(not(windows))]
fn is_installed(_prog_id: &str) -> bool {
    false
}

/// Kills the Office process whose id the script recorded, if any.
fn kill_started_office(pid_file: &Path) {
    let Some(pid) = fs::read_to_string(pid_file)
        .ok()
        .and_then(|text| text.trim().parse::<u32>().ok())
    else {
        return;
    };
    tracing::warn!(
        pid,
        "killing Office process left behind by a failed conversion"
    );
    let mut command = std::process::Command::new("taskkill");
    command.args(["/F", "/PID", &pid.to_string()]);
    tools::hide_console_window(&mut command);
    let _ = command.output();
}

fn absolute(path: &Path) -> Result<PathBuf> {
    std::path::absolute(path).map_err(|e| ConvertError::io(path, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_declared_output_has_a_format_code() {
        for app in [App::Word, App::Excel, App::PowerPoint] {
            for &input in app.kind().inputs() {
                for target in outputs_for(input) {
                    assert!(
                        app.format_code(target).is_some(),
                        "{app:?}: {input} -> {target} has no Office format code"
                    );
                }
            }
        }
    }

    #[test]
    fn respects_engine_choice() {
        let mut options = ConvertOptions::default();
        assert!(MsOfficeConverter::word().is_enabled(&options));
        options.office.engine = OfficeEngine::LibreOffice;
        assert!(!MsOfficeConverter::word().is_enabled(&options));
    }
}
