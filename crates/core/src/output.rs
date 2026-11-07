//! Choosing where converted files are written.

use std::path::{Path, PathBuf};

use crate::Format;

/// Computes a free output path for converting `input` to `target`.
///
/// The file goes into `output_dir`, or next to `input` when `None`. Existing
/// files are never overwritten: `photo.jpg` becomes `photo (1).jpg`,
/// `photo (2).jpg`, … until a free name is found.
pub fn output_path(input: &Path, target: Format, output_dir: Option<&Path>) -> PathBuf {
    let dir = output_dir
        .map(Path::to_path_buf)
        .or_else(|| input.parent().map(Path::to_path_buf))
        .unwrap_or_default();
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = target.extension();

    let candidate = dir.join(format!("{stem}.{ext}"));
    if !candidate.exists() {
        return candidate;
    }

    (1u32..)
        .map(|n| dir.join(format!("{stem} ({n}).{ext}")))
        .find(|path| !path.exists())
        .expect("an unused file name always exists")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn replaces_extension_next_to_input() {
        let path = output_path(Path::new("some/dir/photo.png"), Format::Jpeg, None);
        assert_eq!(path, Path::new("some/dir/photo.jpg"));
    }

    #[test]
    fn honours_output_dir() {
        let path = output_path(
            Path::new("a/photo.png"),
            Format::Bmp,
            Some(Path::new("out")),
        );
        assert_eq!(path, Path::new("out/photo.bmp"));
    }

    #[test]
    fn appends_counter_when_name_is_taken() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("photo.png");
        fs::write(dir.path().join("photo.jpg"), b"").unwrap();
        fs::write(dir.path().join("photo (1).jpg"), b"").unwrap();

        let path = output_path(&input, Format::Jpeg, None);
        assert_eq!(path, dir.path().join("photo (2).jpg"));
    }
}
