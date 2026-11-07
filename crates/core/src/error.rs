//! Error type shared by every part of the engine.

use std::error::Error as StdError;
use std::io;
use std::path::PathBuf;

use crate::Format;

/// Boxed error used to carry library-specific failures (image decoder, …).
pub type BoxError = Box<dyn StdError + Send + Sync + 'static>;

/// Convenience alias used throughout the crate.
pub type Result<T, E = ConvertError> = std::result::Result<T, E>;

/// Everything that can go wrong while converting a file.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConvertError {
    /// The input file's format could not be recognised.
    #[error("cannot recognise the format of '{}'", path.display())]
    UnknownFormat {
        /// The offending input file.
        path: PathBuf,
    },

    /// No registered converter handles this pair of formats.
    #[error("converting {from} to {to} is not supported")]
    Unsupported {
        /// Detected input format.
        from: Format,
        /// Requested output format.
        to: Format,
    },

    /// A converter exists but cannot run on this machine
    /// (e.g. an external tool such as Pandoc is missing).
    #[error("{converter} is unavailable: {reason}")]
    Unavailable {
        /// Identifier of the converter, see [`crate::Converter::id`].
        converter: &'static str,
        /// Human-readable explanation, ideally with a hint on how to fix it.
        reason: String,
    },

    /// Reading or writing a file failed at the OS level.
    #[error("I/O error on '{}'", path.display())]
    Io {
        /// File or directory involved.
        path: PathBuf,
        /// Underlying OS error.
        #[source]
        source: io::Error,
    },

    /// The input file exists but its content could not be decoded.
    #[error("cannot read '{}'", path.display())]
    Decode {
        /// The input file.
        path: PathBuf,
        /// Decoder error.
        #[source]
        source: BoxError,
    },

    /// Producing the output file failed.
    #[error("cannot write '{}'", path.display())]
    Encode {
        /// The output file being written.
        path: PathBuf,
        /// Encoder error.
        #[source]
        source: BoxError,
    },
}

impl ConvertError {
    /// Shorthand for [`ConvertError::Io`].
    pub(crate) fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// Shorthand for [`ConvertError::Decode`].
    pub(crate) fn decode(path: impl Into<PathBuf>, source: impl Into<BoxError>) -> Self {
        Self::Decode {
            path: path.into(),
            source: source.into(),
        }
    }

    /// Shorthand for [`ConvertError::Encode`].
    pub(crate) fn encode(path: impl Into<PathBuf>, source: impl Into<BoxError>) -> Self {
        Self::Encode {
            path: path.into(),
            source: source.into(),
        }
    }
}
