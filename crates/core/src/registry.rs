//! The set of converters known to the application.

use std::collections::BTreeSet;

use crate::converters::{HtmlToMarkdownConverter, ImageConverter, MarkdownConverter};
use crate::{ConvertError, Converter, Format, Result};

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
            .register(MarkdownConverter)
            .register(HtmlToMarkdownConverter);
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
        let targets: BTreeSet<Format> = self
            .converters()
            .filter(|c| c.availability().is_available())
            .flat_map(|c| c.outputs(input))
            .filter(|&target| target != input)
            .collect();
        targets.into_iter().collect()
    }

    /// Formats that *every* format in `inputs` can be converted to – used when
    /// the user selects several files at once.
    ///
    /// A target equal to one of the inputs is still offered as long as the
    /// other inputs support it (e.g. PNG + JPEG → PNG converts only the JPEG
    /// and skips the PNG). It is not offered when *all* inputs already have
    /// that format, since there would be nothing to do.
    pub fn common_targets(&self, inputs: &[Format]) -> Vec<Format> {
        let distinct: BTreeSet<Format> = inputs.iter().copied().collect();
        let mut remaining = distinct.iter().copied();
        let Some(first) = remaining.next() else {
            return Vec::new();
        };

        // A file "supports" its own format as a no-op target.
        let with_self = |format: Format| -> BTreeSet<Format> {
            let mut set: BTreeSet<Format> = self.targets_for(format).into_iter().collect();
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
        common.into_iter().collect()
    }

    /// Finds the converter for `from` → `to`.
    ///
    /// # Errors
    /// - [`ConvertError::Unsupported`] if no converter handles the pair.
    /// - [`ConvertError::Unavailable`] if converters exist but none can run;
    ///   the first one's reason is reported.
    pub fn find(&self, from: Format, to: Format) -> Result<&dyn Converter> {
        let mut first_unavailable = None;

        for converter in self.converters().filter(|c| c.supports(from, to)) {
            match converter.availability() {
                crate::Availability::Available => return Ok(converter),
                crate::Availability::Unavailable { reason } => {
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
    use crate::{Availability, Progress, Request};

    /// Fake converter: PNG → JPEG only, with configurable availability.
    #[derive(Debug)]
    struct Fake {
        available: bool,
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
                Availability::Unavailable {
                    reason: "missing tool".into(),
                }
            }
        }
        fn convert(&self, _: &Request<'_>, _: &dyn Progress) -> Result<()> {
            Ok(())
        }
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
                .common_targets(&[Format::Png, Format::Markdown])
                .is_empty()
        );
    }
}
