//! Turning errors into one-line, human-readable messages.

use std::error::Error;

/// Formats an error together with its chain of causes:
/// `"cannot read 'a.png': Format error decoding Png: invalid signature"`.
pub fn describe_error(err: &dyn Error) -> String {
    let mut message = err.to_string();
    let mut source = err.source();
    while let Some(cause) = source {
        let cause_text = cause.to_string();
        // Some libraries repeat their cause in their own message; skip duplicates.
        if !message.contains(&cause_text) {
            message.push_str(": ");
            message.push_str(&cause_text);
        }
        source = cause.source();
    }
    message
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;
    use std::io;

    /// Minimal error type with a source, to exercise the chain walking.
    #[derive(Debug)]
    struct Outer(io::Error);

    impl fmt::Display for Outer {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("cannot read 'a.png'")
        }
    }

    impl Error for Outer {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            Some(&self.0)
        }
    }

    #[test]
    fn joins_the_cause_chain() {
        let err = Outer(io::Error::other("disk on fire"));
        assert_eq!(describe_error(&err), "cannot read 'a.png': disk on fire");
    }

    #[test]
    fn plain_errors_are_unchanged() {
        let err = io::Error::other("boom");
        assert_eq!(describe_error(&err), "boom");
    }
}
