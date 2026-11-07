//! Progress reporting from converters back to the caller (CLI, UI).

/// Receives progress updates while a file is converted.
///
/// Implemented for any `Fn(f32)` closure, so callers can simply pass
/// `&|fraction| println!("{fraction}")`.
pub trait Progress: Send + Sync {
    /// Reports how much of the current file is done, from `0.0` to `1.0`.
    fn set(&self, fraction: f32);
}

impl<F> Progress for F
where
    F: Fn(f32) + Send + Sync,
{
    fn set(&self, fraction: f32) {
        self(fraction);
    }
}

/// A [`Progress`] that ignores every update.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoProgress;

impl Progress for NoProgress {
    fn set(&self, _fraction: f32) {}
}
