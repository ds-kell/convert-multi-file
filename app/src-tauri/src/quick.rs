//! Quick-convert mode: conversions requested from the Explorer context menu.
//!
//! Explorer starts one `ConvertApp.exe --to <fmt> "<file>"` process **per
//! selected file**. The single-instance plugin forwards each of them to the
//! first process, which pushes them into a [`QuickQueue`]. The quick window
//! then calls [`take_quick_requests`], which waits until no new request has
//! arrived for [`SETTLE_TIME`] and returns everything gathered so far, grouped
//! by target format, so 20 selected files become one batch in one window.

use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::args::QuickRequest;
use crate::dto::FormatInfo;
use crate::window::QUICK_LABEL;

/// How long the queue must stay quiet before a batch is released. Explorer
/// launches the per-file processes within a few dozen milliseconds.
pub const SETTLE_TIME: Duration = Duration::from_millis(300);

/// Event telling the quick window that new requests are waiting.
pub const PENDING_EVENT: &str = "quick://pending";

/// Requests waiting to be picked up by the quick window.
#[derive(Debug)]
pub struct QuickQueue {
    state: Mutex<QueueState>,
}

#[derive(Debug)]
struct QueueState {
    requests: Vec<QuickRequest>,
    last_push: Instant,
}

/// Files to convert to one target, sent to the UI.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuickBatch {
    /// Target format.
    pub target: FormatInfo,
    /// Input files, without duplicates, in arrival order.
    pub paths: Vec<PathBuf>,
}

impl Default for QuickQueue {
    fn default() -> Self {
        Self {
            state: Mutex::new(QueueState {
                requests: Vec::new(),
                last_push: Instant::now(),
            }),
        }
    }
}

impl QuickQueue {
    /// Adds a request and restarts the settle timer.
    pub fn push(&self, request: QuickRequest) {
        let mut state = self.lock();
        state.requests.push(request);
        state.last_push = Instant::now();
    }

    /// Blocks until the queue has been quiet for `settle`, then drains it and
    /// returns the requests merged into one batch per target.
    pub fn take_settled(&self, settle: Duration) -> Vec<QuickBatch> {
        loop {
            let remaining = {
                let mut state = self.lock();
                let quiet_for = state.last_push.elapsed();
                if quiet_for >= settle {
                    return merge(std::mem::take(&mut state.requests));
                }
                settle - quiet_for
            };
            // Sleep outside the lock so pushes are never blocked.
            thread::sleep(remaining);
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, QueueState> {
        // A panic while holding the lock cannot leave the queue inconsistent
        // (every update is a single push/assignment), so recover from poison.
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Groups requests by target, keeping first-seen order and dropping
/// duplicate paths (e.g. the same file right-clicked twice).
fn merge(requests: Vec<QuickRequest>) -> Vec<QuickBatch> {
    let mut batches: Vec<QuickBatch> = Vec::new();
    for request in requests {
        let target = FormatInfo::from(request.target);
        let index = match batches.iter().position(|b| b.target.id == target.id) {
            Some(index) => index,
            None => {
                batches.push(QuickBatch {
                    target,
                    paths: Vec::new(),
                });
                batches.len() - 1
            }
        };
        let paths = &mut batches[index].paths;
        for file in request.files {
            if !paths.contains(&file) {
                paths.push(file);
            }
        }
    }
    batches
}

/// Queues `request` and makes sure the quick window is open and notified.
pub fn enqueue(app: &AppHandle, request: QuickRequest) {
    app.state::<QuickQueue>().push(request);
    if let Err(err) = crate::window::show_quick(app) {
        eprintln!("cannot open the quick-convert window: {err}");
    }
    // Ignored if the window is still loading: it takes the queue on start-up.
    let _ = app.emit_to(QUICK_LABEL, PENDING_EVENT, ());
}

/// Returns the pending batches once the queue has settled. Called by the quick
/// window on start-up and after each [`PENDING_EVENT`].
#[tauri::command]
pub async fn take_quick_requests(app: AppHandle) -> Result<Vec<QuickBatch>, String> {
    // `take_settled` sleeps, so run it on a blocking thread. `State` borrows
    // cannot move there, but a cloned `AppHandle` can look the queue up.
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<QuickQueue>().take_settled(SETTLE_TIME)
    })
    .await
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use convertapp_core::Format;

    fn request(target: Format, files: &[&str]) -> QuickRequest {
        QuickRequest {
            target,
            files: files.iter().map(PathBuf::from).collect(),
        }
    }

    #[test]
    fn merge_groups_by_target_and_drops_duplicates() {
        let batches = merge(vec![
            request(Format::Jpeg, &["a.png"]),
            request(Format::Html, &["n.md"]),
            request(Format::Jpeg, &["b.png", "a.png"]),
        ]);
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].target.id, "jpg");
        assert_eq!(
            batches[0].paths,
            [PathBuf::from("a.png"), PathBuf::from("b.png")]
        );
        assert_eq!(batches[1].target.id, "html");
    }

    #[test]
    fn take_waits_for_the_queue_to_settle() {
        let queue = QuickQueue::default();
        let settle = Duration::from_millis(80);

        thread::scope(|scope| {
            scope.spawn(|| {
                for name in ["1.png", "2.png", "3.png"] {
                    queue.push(request(Format::Jpeg, &[name]));
                    thread::sleep(Duration::from_millis(20));
                }
            });
            thread::sleep(Duration::from_millis(5));
            let batches = queue.take_settled(settle);
            assert_eq!(batches.len(), 1);
            assert_eq!(
                batches[0].paths.len(),
                3,
                "all pushes within the window are batched"
            );
        });
    }

    #[test]
    fn take_on_an_idle_queue_returns_nothing() {
        let queue = QuickQueue::default();
        assert!(queue.take_settled(Duration::ZERO).is_empty());
    }
}
