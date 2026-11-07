//! Tauri commands callable from the frontend with `invoke(...)`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use convertapp_core::{
    ConvertOptions, Format, Job, Registry, convert, error_chain, merge_images_to_pdf,
};
use tauri::State;
use tauri::ipc::Channel;

use crate::dto::{ConvertEvent, FileInfo, FormatInfo, Inspection, LockedInfo, Settings};

/// Shared application state, registered with `Builder::manage`.
#[derive(Debug)]
pub struct AppState {
    /// Converters available in this build. Shared with worker threads.
    pub registry: Arc<Registry>,
}

/// Detects the format of each file and the targets they all support.
///
/// Called whenever the file list changes. Only file headers are read, so this
/// is fast even for large files.
#[tauri::command]
pub async fn inspect_files(
    state: State<'_, AppState>,
    paths: Vec<PathBuf>,
) -> Result<Inspection, String> {
    let registry = Arc::clone(&state.registry);

    // File access is blocking I/O: keep it off the async runtime.
    tauri::async_runtime::spawn_blocking(move || inspect(&registry, paths))
        .await
        .map_err(|e| e.to_string())
}

/// Converts `paths` to `target`, reporting progress through `on_event`.
///
/// Files are processed one by one on a background thread. A failing file does
/// not stop the batch; the final [`ConvertEvent::Finished`] carries the totals.
#[tauri::command]
pub async fn convert_files(
    state: State<'_, AppState>,
    paths: Vec<PathBuf>,
    target: String,
    settings: Settings,
    on_event: Channel<ConvertEvent>,
) -> Result<(), String> {
    let target: Format = target.parse().map_err(|e| format!("{e}"))?;
    let registry = Arc::clone(&state.registry);

    tauri::async_runtime::spawn_blocking(move || {
        run_batch(&registry, &paths, target, &settings, &on_event);
    })
    .await
    .map_err(|e| e.to_string())
}

/// Implementation of [`inspect_files`], separated for readability.
fn inspect(registry: &Registry, paths: Vec<PathBuf>) -> Inspection {
    // Formats of files that can be converted now / that were recognised at all.
    let mut convertible = Vec::new();
    let mut recognised = Vec::new();

    let files = paths
        .into_iter()
        .map(|path| {
            let name = file_name(&path);
            let (format, error) = match Format::detect(&path) {
                Ok(Some(format)) => {
                    recognised.push(format);
                    let error = if registry.targets_for(format).is_empty() {
                        // Say *why*: a missing tool is fixable, unsupported is not.
                        Some(
                            match registry.locked_targets(&[format]).into_iter().next() {
                                Some(locked) => locked.reason,
                                None => "Chưa hỗ trợ chuyển đổi định dạng này".to_owned(),
                            },
                        )
                    } else {
                        convertible.push(format);
                        None
                    };
                    (Some(format.into()), error)
                }
                Ok(None) => (None, Some("Không nhận diện được định dạng".to_owned())),
                Err(err) => (None, Some(error_chain(&err))),
            };
            FileInfo {
                path,
                name,
                format,
                error,
            }
        })
        .collect();

    let targets = registry
        .common_targets(&convertible)
        .into_iter()
        .map(FormatInfo::from)
        .collect();
    let locked = registry
        .locked_targets(&recognised)
        .into_iter()
        .map(|locked| LockedInfo {
            format: locked.format.into(),
            reason: locked.reason,
        })
        .collect();

    Inspection {
        files,
        targets,
        locked,
    }
}

/// Converts every file and streams events. Send errors (the window was
/// closed) are ignored: there is nobody left to notify.
fn run_batch(
    registry: &Registry,
    paths: &[PathBuf],
    target: Format,
    settings: &Settings,
    events: &Channel<ConvertEvent>,
) {
    let options = settings.to_options();

    // Merging only makes sense for images; a stale checkbox must not turn a
    // batch of Markdown → PDF conversions into a failed merge.
    let all_images = || {
        paths.iter().all(|p| {
            matches!(Format::detect(p), Ok(Some(f)) if f.category() == convertapp_core::Category::Image)
        })
    };
    if target == Format::Pdf && settings.merge_pdf && paths.len() > 1 && all_images() {
        run_merge(paths, settings, &options, events);
        return;
    }

    let (mut converted, mut skipped, mut failed) = (0, 0, 0);

    for (index, path) in paths.iter().enumerate() {
        let _ = events.send(ConvertEvent::Started { index });

        // A multi-selection may contain files already in the target format.
        if let Ok(Some(format)) = Format::detect(path)
            && format == target
        {
            skipped += 1;
            let reason = format!("Đã là {target}");
            let _ = events.send(ConvertEvent::Skipped { index, reason });
            continue;
        }

        let mut job = Job::new(path, target).with_options(options.clone());
        if let Some(dir) = &settings.output_dir {
            job = job.with_output_dir(dir);
        }

        let progress = |fraction: f32| {
            let _ = events.send(ConvertEvent::Progress { index, fraction });
        };

        let event = match convert(registry, &job, &progress) {
            Ok(output) => {
                converted += 1;
                ConvertEvent::Done { index, output }
            }
            Err(err) => {
                failed += 1;
                ConvertEvent::Failed {
                    index,
                    message: error_chain(&err),
                }
            }
        };
        let _ = events.send(event);
    }

    let _ = events.send(ConvertEvent::Finished {
        converted,
        skipped,
        failed,
    });
}

/// Merges all images into one PDF. Every row shows the same progress and the
/// same result, since they produce a single file together.
fn run_merge(
    paths: &[PathBuf],
    settings: &Settings,
    options: &ConvertOptions,
    events: &Channel<ConvertEvent>,
) {
    let all = 0..paths.len();
    for index in all.clone() {
        let _ = events.send(ConvertEvent::Started { index });
    }

    let progress = |fraction: f32| {
        for index in all.clone() {
            let _ = events.send(ConvertEvent::Progress { index, fraction });
        }
    };

    let result = merge_images_to_pdf(paths, settings.output_dir.as_deref(), options, &progress);
    let (converted, failed) = match &result {
        Ok(_) => (paths.len(), 0),
        Err(_) => (0, paths.len()),
    };
    for index in all {
        let event = match &result {
            Ok(output) => ConvertEvent::Done {
                index,
                output: output.clone(),
            },
            Err(err) => ConvertEvent::Failed {
                index,
                message: error_chain(err),
            },
        };
        let _ = events.send(event);
    }

    let _ = events.send(ConvertEvent::Finished {
        converted,
        skipped: 0,
        failed,
    });
}

/// File name for display, falling back to the full path.
fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn inspect_reports_formats_errors_and_common_targets() {
        let dir = tempfile::tempdir().unwrap();
        let md = dir.path().join("a.md");
        let unknown = dir.path().join("b.xyz");
        fs::write(&md, "# hi").unwrap();
        fs::write(&unknown, "??").unwrap();

        let registry = Registry::with_defaults();
        let result = inspect(&registry, vec![md, unknown, dir.path().join("missing.png")]);

        assert_eq!(result.files.len(), 3);
        assert_eq!(result.files[0].format.as_ref().unwrap().id, "md");
        assert!(result.files[0].error.is_none());
        assert!(result.files[1].error.is_some());
        assert!(result.files[2].error.is_some());
        // Only the Markdown file is recognised, so its targets are offered
        // (plus PDF/PNG/Word depending on installed tools); never JPEG.
        let targets: Vec<_> = result.targets.iter().map(|t| t.id).collect();
        assert!(targets.contains(&"html"), "{targets:?}");
        assert!(!targets.contains(&"jpg"), "{targets:?}");
    }
}
