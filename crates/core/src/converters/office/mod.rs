//! Office documents (Word, Excel, PowerPoint and their OpenDocument
//! equivalents), converted by an installed office suite.
//!
//! Two engines implement the same conversion table:
//! - [Microsoft Office](msoffice) through COM automation (Windows only),
//!   which gives output identical to "Save As" in Word/Excel/PowerPoint;
//! - [LibreOffice](libreoffice) in headless mode, free and cross-platform.
//!
//! Microsoft Office is preferred when installed; [`crate::OfficeEngine`]
//! lets the user force one of them.

pub(crate) mod libreoffice;
pub(crate) mod msoffice;

use crate::{ConvertOptions, Format, OfficeEngine};

/// The three kinds of office documents. Conversions stay within a kind
/// (plus PDF), e.g. a spreadsheet never becomes a presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// Word processing.
    Text,
    /// Spreadsheets.
    Spreadsheet,
    /// Slides.
    Presentation,
}

impl Kind {
    /// Formats of this kind that can be opened.
    pub(crate) fn inputs(self) -> &'static [Format] {
        match self {
            Kind::Text => &[Format::Docx, Format::Doc, Format::Odt, Format::Rtf],
            Kind::Spreadsheet => &[Format::Xlsx, Format::Xls, Format::Ods, Format::Csv],
            Kind::Presentation => &[Format::Pptx, Format::Ppt, Format::Odp],
        }
    }

    /// Formats documents of this kind can be saved as.
    pub(crate) fn outputs(self) -> &'static [Format] {
        match self {
            // No HTML: office suites write it with a side folder of images
            // named after the temporary file. Word → HTML goes through
            // Pandoc instead (single self-contained page).
            Kind::Text => &[
                Format::Pdf,
                Format::Docx,
                Format::Odt,
                Format::Rtf,
                Format::Txt,
            ],
            Kind::Spreadsheet => &[Format::Pdf, Format::Xlsx, Format::Ods, Format::Csv],
            Kind::Presentation => &[Format::Pdf, Format::Pptx, Format::Odp],
        }
    }

    /// The kind `format` belongs to as an *input*.
    pub(crate) fn of(format: Format) -> Option<Kind> {
        [Kind::Text, Kind::Spreadsheet, Kind::Presentation]
            .into_iter()
            .find(|kind| kind.inputs().contains(&format))
    }
}

/// Every office format that can be opened.
pub(crate) const ALL_INPUTS: &[Format] = &[
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
];

/// Targets for an office `input` (never the input itself).
pub(crate) fn outputs_for(input: Format) -> Vec<Format> {
    Kind::of(input)
        .map(|kind| {
            kind.outputs()
                .iter()
                .copied()
                .filter(|&f| f != input)
                .collect()
        })
        .unwrap_or_default()
}

/// Whether `engine` may be used under the user's [`OfficeEngine`] choice.
pub(crate) fn allowed(options: &ConvertOptions, engine: OfficeEngine) -> bool {
    match options.office.engine {
        OfficeEngine::Auto => true,
        chosen => chosen == engine,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions_stay_within_a_kind() {
        let xlsx = outputs_for(Format::Xlsx);
        assert!(xlsx.contains(&Format::Pdf) && xlsx.contains(&Format::Csv));
        assert!(!xlsx.contains(&Format::Docx) && !xlsx.contains(&Format::Xlsx));

        assert!(outputs_for(Format::Doc).contains(&Format::Docx));
        assert!(outputs_for(Format::Ppt).contains(&Format::Pptx));
        assert!(outputs_for(Format::Png).is_empty());
    }

    #[test]
    fn every_input_has_a_kind() {
        for &format in ALL_INPUTS {
            assert!(Kind::of(format).is_some(), "{format}");
        }
    }
}
