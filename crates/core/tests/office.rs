//! Office documents with both engines: Microsoft Office (COM) and
//! LibreOffice. Each test runs once per installed engine and skips engines
//! that are missing, so it passes on any machine.
//!
//! Inputs are generated at test time: CSV is written directly, Word and
//! PowerPoint files are produced by Pandoc (when installed).

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use convertapp_core::tools::PANDOC;
use convertapp_core::{
    ConvertOptions, Format, Job, NoProgress, OfficeEngine, OfficeOptions, Registry, convert,
};

/// The engines available on this machine for `from` → `to`.
fn engines(from: Format, to: Format) -> Vec<OfficeEngine> {
    let registry = Registry::with_defaults();
    let available: Vec<OfficeEngine> = [OfficeEngine::MicrosoftOffice, OfficeEngine::LibreOffice]
        .into_iter()
        .filter(|&engine| registry.find_for(from, to, &options(engine)).is_ok())
        .collect();
    if available.is_empty() {
        eprintln!("skipped: no office suite for {from} -> {to}");
    }
    available
}

fn options(engine: OfficeEngine) -> ConvertOptions {
    ConvertOptions {
        office: OfficeOptions { engine },
        ..ConvertOptions::default()
    }
}

fn run(input: &Path, target: Format, engine: OfficeEngine) -> PathBuf {
    let job = Job::new(input, target).with_options(options(engine));
    convert(&Registry::with_defaults(), &job, &NoProgress)
        .unwrap_or_else(|e| panic!("{engine:?}: {} -> {target}: {e}", input.display()))
}

/// A fresh folder per engine so outputs never collide.
fn workspace(engine: OfficeEngine) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("office-{engine:?}-"))
        .tempdir()
        .unwrap()
}

fn assert_pdf(path: &Path) {
    let bytes = fs::read(path).unwrap();
    assert!(
        bytes.starts_with(b"%PDF-"),
        "{} is not a PDF",
        path.display()
    );
    assert!(bytes.len() > 500);
}

/// The text content of one XML entry of an OOXML/ODF file (both are ZIP
/// archives), with tags removed. Office may split a word across several
/// runs (`<w:t>Xin ch</w:t>…<w:t>ào</w:t>`), so tags must go before searching.
fn zip_text(path: &Path, name: &str) -> String {
    let mut archive = zip::ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
    let mut xml = String::new();
    archive
        .by_name(name)
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();

    let mut text = String::with_capacity(xml.len());
    let mut in_tag = false;
    for c in xml.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }
    text
}

/// Writes Markdown to `<dir>/<name>.md` and converts it with Pandoc to
/// `<dir>/<name>.<ext>`. `None` if Pandoc is not installed.
fn pandoc_file(dir: &Path, name: &str, ext: &str, markdown: &str) -> Option<PathBuf> {
    let pandoc = PANDOC.locate()?;
    let md = dir.join(format!("{name}.md"));
    fs::write(&md, markdown).unwrap();
    let out = dir.join(format!("{name}.{ext}"));
    let status = Command::new(pandoc)
        .arg(&md)
        .arg("-o")
        .arg(&out)
        .status()
        .unwrap();
    assert!(status.success());
    fs::remove_file(md).unwrap();
    Some(out)
}

const REPORT_MD: &str = "\
# Báo cáo quý

Doanh thu **tăng** so với quý trước: ầ ẩ ỡ ự Đ đ.

| Hạng mục | Số lượng |
|---|---|
| Ảnh | 12 |
| Tài liệu | 3 |
";

const CSV: &str = "Hạng mục,Số lượng\nẢnh,12\nTài liệu,3\n";

#[test]
fn word_document_to_pdf_odt_and_text() {
    for engine in engines(Format::Docx, Format::Pdf) {
        let dir = workspace(engine);
        let Some(docx) = pandoc_file(dir.path(), "report", "docx", REPORT_MD) else {
            eprintln!("skipped: Pandoc is needed to create the test document");
            return;
        };

        assert_pdf(&run(&docx, Format::Pdf, engine));

        let odt = run(&docx, Format::Odt, engine);
        assert!(
            zip_text(&odt, "content.xml").contains("Báo cáo quý"),
            "{engine:?}"
        );

        let txt =
            fs::read_to_string(run(&docx, Format::Txt, engine)).expect("text output is UTF-8");
        assert!(txt.contains("ầ ẩ ỡ ự Đ đ"), "{engine:?}: {txt}");
    }
}

#[test]
fn rtf_opens_as_a_word_document() {
    for engine in engines(Format::Rtf, Format::Docx) {
        let dir = workspace(engine);
        let rtf = dir.path().join("note.rtf");
        // "Xin chào" with the à written as an RTF Unicode escape.
        fs::write(
            &rtf,
            r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0 Xin ch\u224?o\par}",
        )
        .unwrap();

        let docx = run(&rtf, Format::Docx, engine);
        assert!(
            zip_text(&docx, "word/document.xml").contains("Xin chào"),
            "{engine:?}"
        );
    }
}

#[test]
fn csv_to_spreadsheet_and_back() {
    for engine in engines(Format::Csv, Format::Xlsx) {
        let dir = workspace(engine);
        let csv = dir.path().join("data.csv");
        fs::write(&csv, CSV).unwrap();

        let xlsx = run(&csv, Format::Xlsx, engine);
        assert_eq!(Format::detect(&xlsx).unwrap(), Some(Format::Xlsx));
        assert!(
            zip_text(&xlsx, "xl/sharedStrings.xml").contains("Tài liệu"),
            "{engine:?}"
        );

        assert_pdf(&run(&xlsx, Format::Pdf, engine));

        // Back to CSV (as data2.csv's sibling): the text survives in UTF-8.
        fs::remove_file(&csv).unwrap();
        let back = fs::read_to_string(run(&xlsx, Format::Csv, engine)).unwrap();
        let back = back.trim_start_matches('\u{feff}');
        // Engines differ in quoting ("Tài liệu",3 vs Tài liệu,3); compare
        // the values only.
        let unquoted = back.replace('"', "");
        assert!(unquoted.contains("Tài liệu,3"), "{engine:?}: {back}");
    }
}

#[test]
fn presentation_to_pdf_and_odp() {
    for engine in engines(Format::Pptx, Format::Pdf) {
        let dir = workspace(engine);
        let Some(pptx) = pandoc_file(
            dir.path(),
            "slides",
            "pptx",
            "# Trang 1\n\nXin chào\n\n# Trang 2\n\n- Ý một\n- Ý hai\n",
        ) else {
            eprintln!("skipped: Pandoc is needed to create the test presentation");
            return;
        };

        assert_pdf(&run(&pptx, Format::Pdf, engine));
        let odp = run(&pptx, Format::Odp, engine);
        assert!(
            zip_text(&odp, "content.xml").contains("Xin chào"),
            "{engine:?}"
        );
    }
}

#[test]
fn damaged_documents_fail_cleanly() {
    for engine in engines(Format::Docx, Format::Pdf) {
        let dir = workspace(engine);
        let broken = dir.path().join("broken.docx");
        // A ZIP signature followed by garbage: looks like OOXML, isn't.
        fs::write(&broken, b"PK\x03\x04 definitely not a word document").unwrap();

        let job = Job::new(&broken, Format::Pdf).with_options(options(engine));
        let err = convert(&Registry::with_defaults(), &job, &NoProgress)
            .expect_err("a damaged file must not convert");
        assert!(err.to_string().contains("failed"), "{engine:?}: {err}");

        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(
            leftovers.len(),
            1,
            "{engine:?}: no output left behind: {leftovers:?}"
        );
    }
}

/// Reads Word's `DisableConvertPdfWarning` option (None when absent), to
/// check the conversion leaves it exactly as it found it.
#[cfg(windows)]
fn word_pdf_warning_option() -> Option<String> {
    let output = Command::new("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Office\16.0\Word\Options",
            "/v",
            "DisableConvertPdfWarning",
        ])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[test]
fn pdf_becomes_an_editable_word_document() {
    if convertapp_core::browser::locate().is_none() {
        eprintln!("skipped: a browser is needed to create the test PDF");
        return;
    }
    for engine in engines(Format::Pdf, Format::Docx) {
        let dir = workspace(engine);
        // A real text PDF, printed by the browser from HTML.
        let html = dir.path().join("report.html");
        fs::write(
            &html,
            "<!doctype html><meta charset='utf-8'><h1>Báo cáo quý</h1>\
             <p>Doanh thu tăng: ầ ẩ ỡ ự Đ đ.</p>\
             <table border='1'><tr><th>Hạng mục</th><th>Số lượng</th></tr>\
             <tr><td>Tài liệu</td><td>3</td></tr></table>",
        )
        .unwrap();
        let pdf = run(&html, Format::Pdf, engine);
        fs::remove_file(&html).unwrap();

        #[cfg(windows)]
        let option_before = word_pdf_warning_option();

        let docx = run(&pdf, Format::Docx, engine);
        let text = zip_text(&docx, "word/document.xml");
        assert!(text.contains("Báo cáo quý"), "{engine:?}: {text}");
        assert!(text.contains("ầ ẩ ỡ ự Đ đ"), "{engine:?}: Vietnamese kept");
        assert!(text.contains("Tài liệu"), "{engine:?}: table text kept");

        if engine == OfficeEngine::MicrosoftOffice {
            let txt = fs::read_to_string(run(&pdf, Format::Txt, engine)).unwrap();
            assert!(txt.contains("Doanh thu"), "{txt}");
            #[cfg(windows)]
            assert_eq!(
                word_pdf_warning_option(),
                option_before,
                "Word's PDF warning option must be restored"
            );
        }
    }
}

#[test]
fn engine_choice_is_respected() {
    let registry = Registry::with_defaults();
    if let Ok(converter) = registry.find_for(
        Format::Docx,
        Format::Pdf,
        &options(OfficeEngine::LibreOffice),
    ) {
        assert_eq!(converter.id(), "libreoffice");
    }
    if let Ok(converter) = registry.find_for(
        Format::Docx,
        Format::Pdf,
        &options(OfficeEngine::MicrosoftOffice),
    ) {
        assert_eq!(converter.id(), "ms-word");
    }
}

#[test]
fn microsoft_office_is_only_suggested_on_windows() {
    let registry = Registry::with_defaults();
    let suggested: Vec<&str> = registry
        .locked_targets(&[Format::Docx])
        .iter()
        .flat_map(|locked| locked.alternatives.iter().flatten())
        .map(|req| req.name)
        .collect();
    // Office for Mac / Linux has no COM automation to drive.
    if !cfg!(windows) {
        assert!(!suggested.contains(&"Microsoft Office"), "{suggested:?}");
    }
}
