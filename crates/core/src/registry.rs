//! The set of converters known to the application.

use std::collections::BTreeSet;

use crate::converters::{
    HtmlToMarkdownConverter, ImageConverter, LibreOfficeConverter, MarkdownConverter,
    MarkdownToPdfConverter, MsOfficeConverter, PandocConverter, SvgConverter, WebConverter,
};
use crate::{Availability, ConvertError, ConvertOptions, Converter, Format, Requirement, Result};

/// A conversion target that exists but cannot be used right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedTarget {
    /// The target format.
    pub format: Format,
    /// Why it is unavailable (English), for logs and the CLI.
    pub reason: String,
    /// Ways to unlock it: installing **all** programs of **any one** inner
    /// list is enough (e.g. `[[LibreOffice], [Microsoft Office]]`, or
    /// `[[Edge], [Pandoc, Typst]]`). Free options come first.
    pub alternatives: Vec<Vec<Requirement>>,
}

/// Holds every registered [`Converter`] and answers questions such as
/// "what can a PNG become?" for the CLI, the UI and the context menu.
#[derive(Debug, Default)]
pub struct Registry {
    converters: Vec<Box<dyn Converter>>,
}

impl Registry {
    /// Creates an empty registry. Mostly useful in tests.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a registry containing all built-in converters.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry
            .register(ImageConverter)
            .register(SvgConverter)
            .register(MarkdownConverter)
            .register(HtmlToMarkdownConverter)
            // External programs last: built-in converters win for shared pairs.
            // The browser comes before Pandoc so Markdown → PDF works with
            // just Edge/Chrome; Pandoc + Typst remain the fallback.
            .register(WebConverter)
            .register(PandocConverter)
            .register(MarkdownToPdfConverter)
            // Office documents: Microsoft Office first (identical to Save As
            // in Word/Excel/PowerPoint), LibreOffice as the free fallback.
            .register(MsOfficeConverter::word())
            .register(MsOfficeConverter::excel())
            .register(MsOfficeConverter::powerpoint())
            .register(LibreOfficeConverter);
        registry
    }

    /// Adds a converter. When several converters handle the same pair of
    /// formats, the one registered first wins.
    pub fn register(&mut self, converter: impl Converter + 'static) -> &mut Self {
        self.converters.push(Box::new(converter));
        self
    }

    /// Iterates over all registered converters, available or not.
    pub fn converters(&self) -> impl Iterator<Item = &dyn Converter> {
        self.converters.iter().map(Box::as_ref)
    }

    /// Formats that `input` can be converted to with the converters that are
    /// currently available. Sorted, without duplicates, never contains `input`.
    pub fn targets_for(&self, input: Format) -> Vec<Format> {
        self.reachable(input, true).into_iter().collect()
    }

    /// Formats that *every* format in `inputs` can be converted to – used when
    /// the user selects several files at once.
    ///
    /// A target equal to one of the inputs is still offered as long as the
    /// other inputs support it (e.g. PNG + JPEG → PNG converts only the JPEG
    /// and skips the PNG). It is not offered when *all* inputs already have
    /// that format, since there would be nothing to do.
    pub fn common_targets(&self, inputs: &[Format]) -> Vec<Format> {
        self.common(inputs, true).into_iter().collect()
    }

    /// Targets the files in `inputs` *could* all be converted to, but which
    /// are unavailable right now (typically because an external program is
    /// not installed), each with the reason. The UI shows them as locked.
    pub fn locked_targets(&self, inputs: &[Format]) -> Vec<LockedTarget> {
        let available = self.common(inputs, true);
        let Some(&first) = inputs.first() else {
            return Vec::new();
        };

        self.common(inputs, false)
            .into_iter()
            .filter(|target| !available.contains(target))
            .map(|format| {
                // `first` might already be `format`; ask about another input.
                let from = inputs
                    .iter()
                    .copied()
                    .find(|&f| f != format)
                    .unwrap_or(first);
                let reason = match self.find(from, format) {
                    Err(ConvertError::Unavailable { reason, .. }) => reason,
                    Err(other) => other.to_string(),
                    Ok(_) => String::new(), // cannot happen: not available above
                };
                LockedTarget {
                    format,
                    reason,
                    alternatives: self.alternatives(from, format),
                }
            })
            .collect()
    }

    /// What each converter able to do `from` → `to` is missing, without
    /// duplicates, free options first.
    fn alternatives(&self, from: Format, to: Format) -> Vec<Vec<Requirement>> {
        let mut alternatives: Vec<Vec<Requirement>> = Vec::new();
        for converter in self.converters().filter(|c| c.supports(from, to)) {
            if let Availability::Unavailable { missing, .. } = converter.availability()
                && !missing.is_empty()
                && !alternatives.contains(&missing)
            {
                alternatives.push(missing);
            }
        }
        // Stable sort keeps the registry's preference among equals.
        alternatives.sort_by_key(|option| !option.iter().all(|req| req.free));
        alternatives
    }

    /// Targets of `input`, optionally only from available converters.
    fn reachable(&self, input: Format, only_available: bool) -> BTreeSet<Format> {
        self.converters()
            .filter(|c| !only_available || c.availability().is_available())
            .flat_map(|c| c.outputs(input))
            .filter(|&target| target != input)
            .collect()
    }

    /// Implementation of [`common_targets`](Self::common_targets), optionally
    /// counting unavailable converters too.
    fn common(&self, inputs: &[Format], only_available: bool) -> BTreeSet<Format> {
        let distinct: BTreeSet<Format> = inputs.iter().copied().collect();
        let mut remaining = distinct.iter().copied();
        let Some(first) = remaining.next() else {
            return BTreeSet::new();
        };

        // A file "supports" its own format as a no-op target.
        let with_self = |format: Format| -> BTreeSet<Format> {
            let mut set = self.reachable(format, only_available);
            set.insert(format);
            set
        };

        let mut common = with_self(first);
        for format in remaining {
            let other = with_self(format);
            common.retain(|target| other.contains(target));
        }
        if distinct.len() == 1 {
            common.remove(&first);
        }
        common
    }

    /// Finds the converter for `from` → `to`.
    ///
    /// # Errors
    /// - [`ConvertError::Unsupported`] if no converter handles the pair.
    /// - [`ConvertError::Unavailable`] if converters exist but none can run;
    ///   the first one's reason is reported.
    pub fn find(&self, from: Format, to: Format) -> Result<&dyn Converter> {
        self.find_for(from, to, &ConvertOptions::default())
    }

    /// Like [`find`](Self::find), but skips converters the user disabled in
    /// `options` (see [`Converter::is_enabled`]).
    ///
    /// # Errors
    /// See [`find`](Self::find).
    pub fn find_for(
        &self,
        from: Format,
        to: Format,
        options: &ConvertOptions,
    ) -> Result<&dyn Converter> {
        let mut first_unavailable = None;

        let candidates = self
            .converters()
            .filter(|c| c.supports(from, to) && c.is_enabled(options));
        for converter in candidates {
            match converter.availability() {
                Availability::Available => return Ok(converter),
                Availability::Unavailable { reason, .. } => {
                    first_unavailable.get_or_insert((converter.id(), reason));
                }
            }
        }

        Err(match first_unavailable {
            Some((converter, reason)) => ConvertError::Unavailable { converter, reason },
            None => ConvertError::Unsupported { from, to },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Progress, Request};

    /// What [`Fake`] needs when unavailable.
    const TOOL: Requirement = Requirement {
        name: "Tool",
        url: "https://example.com/tool",
        install_command: Some("install tool"),
        free: true,
    };

    /// A paid alternative, to check that free options are listed first.
    const PAID: Requirement = Requirement {
        name: "Paid",
        url: "https://example.com/paid",
        install_command: None,
        free: false,
    };

    /// Fake converter: PNG → JPEG only, with configurable availability.
    #[derive(Debug)]
    struct Fake {
        available: bool,
    }

    /// Another PNG → JPEG converter that needs a paid program.
    #[derive(Debug)]
    struct FakePaid;

    impl Converter for FakePaid {
        fn id(&self) -> &'static str {
            "fake-paid"
        }
        fn inputs(&self) -> &[Format] {
            &[Format::Png]
        }
        fn outputs(&self, _input: Format) -> Vec<Format> {
            vec![Format::Jpeg]
        }
        fn availability(&self) -> Availability {
            Availability::missing(vec![PAID])
        }
        fn convert(&self, _: &Request<'_>, _: &dyn Progress) -> Result<()> {
            Ok(())
        }
    }

    impl Converter for Fake {
        fn id(&self) -> &'static str {
            "fake"
        }
        fn inputs(&self) -> &[Format] {
            &[Format::Png]
        }
        fn outputs(&self, input: Format) -> Vec<Format> {
            if input == Format::Png {
                vec![Format::Jpeg]
            } else {
                vec![]
            }
        }
        fn availability(&self) -> Availability {
            if self.available {
                Availability::Available
            } else {
                Availability::missing(vec![TOOL])
            }
        }
        fn convert(&self, _: &Request<'_>, _: &dyn Progress) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn locked_targets_list_what_to_install_free_first() {
        let mut registry = Registry::new();
        // The paid converter is registered first but listed last.
        registry
            .register(FakePaid)
            .register(Fake { available: false });
        let locked = registry.locked_targets(&[Format::Png]);
        assert_eq!(locked.len(), 1);
        assert_eq!(locked[0].format, Format::Jpeg);
        assert_eq!(locked[0].alternatives, [vec![TOOL], vec![PAID]]);
        assert!(locked[0].reason.contains("Paid is not installed"));

        // Once an available converter provides the pair, it is not locked.
        registry.register(Fake { available: true });
        assert!(registry.locked_targets(&[Format::Png]).is_empty());
    }

    #[test]
    fn targets_skip_unavailable_converters() {
        let mut registry = Registry::new();
        registry.register(Fake { available: false });
        assert!(registry.targets_for(Format::Png).is_empty());

        registry.register(Fake { available: true });
        assert_eq!(registry.targets_for(Format::Png), vec![Format::Jpeg]);
    }

    #[test]
    fn find_reports_unsupported_and_unavailable() {
        let mut registry = Registry::new();
        assert!(matches!(
            registry.find(Format::Png, Format::Jpeg),
            Err(ConvertError::Unsupported { .. })
        ));

        registry.register(Fake { available: false });
        assert!(matches!(
            registry.find(Format::Png, Format::Jpeg),
            Err(ConvertError::Unavailable {
                converter: "fake",
                ..
            })
        ));
    }

    #[test]
    fn find_prefers_first_available() {
        let mut registry = Registry::new();
        registry
            .register(Fake { available: false })
            .register(Fake { available: true });
        assert!(registry.find(Format::Png, Format::Jpeg).is_ok());
    }

    #[test]
    fn default_registry_converts_between_images() {
        let registry = Registry::with_defaults();
        let targets = registry.targets_for(Format::Png);
        assert!(targets.contains(&Format::Jpeg));
        assert!(targets.contains(&Format::Ico));
        assert!(!targets.contains(&Format::Png));
    }

    #[test]
    fn common_targets_intersects_and_keeps_own_format() {
        let registry = Registry::with_defaults();
        let common = registry.common_targets(&[Format::Png, Format::Jpeg]);
        assert!(common.contains(&Format::Png));
        assert!(common.contains(&Format::Jpeg));
        assert!(common.contains(&Format::Bmp));
        assert!(registry.common_targets(&[]).is_empty());

        // Same format only: converting to itself would do nothing.
        let same = registry.common_targets(&[Format::Png, Format::Png]);
        assert!(!same.contains(&Format::Png));
        assert!(same.contains(&Format::Jpeg));

        // Unrelated families have nothing in common.
        assert!(
            registry
                .common_targets(&[Format::Svg, Format::Docx])
                .is_empty()
        );
    }
}
