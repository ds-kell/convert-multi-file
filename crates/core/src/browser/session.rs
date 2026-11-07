//! A running headless browser and the page operations ConvertApp needs.

use std::fs;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use base64::Engine;
use serde_json::{Value, json};
use url::Url;

use super::cdp::{Cdp, CdpError};
use crate::{ConvertError, Result};

/// Name used in error messages.
const TOOL: &str = "Browser";
/// Time allowed for the browser to start and open its DevTools port.
const LAUNCH_TIMEOUT: Duration = Duration::from_secs(20);
/// Time allowed for a page (and its images, fonts, scripts) to load.
const LOAD_TIMEOUT: Duration = Duration::from_secs(45);
/// Time allowed for quick commands.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(15);
/// Time allowed to produce a PDF or screenshot of a long page.
const RENDER_TIMEOUT: Duration = Duration::from_secs(120);
/// Tallest screenshot we take, in CSS pixels (Chromium's own limit is 16384).
const MAX_SCREENSHOT_HEIGHT: u32 = 16_384;

/// Page setup for [`Browser::print_pdf`], in inches as CDP expects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PdfSettings {
    /// Paper width (portrait orientation).
    pub paper_width: f64,
    /// Paper height (portrait orientation).
    pub paper_height: f64,
    /// Rotate the paper.
    pub landscape: bool,
    /// Margin on every side.
    pub margin: f64,
    /// Print background colours and images, as on screen.
    pub print_background: bool,
}

/// A headless Edge/Chrome with a temporary profile, closed on drop.
pub struct Browser {
    child: Child,
    cdp: Cdp,
    /// Kept alive for the browser's lifetime, deleted on drop.
    _profile: tempfile::TempDir,
}

impl std::fmt::Debug for Browser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Browser")
            .field("pid", &self.child.id())
            .finish_non_exhaustive()
    }
}

impl Browser {
    /// Starts `exe` headless and connects to it.
    ///
    /// # Errors
    /// [`ConvertError::External`] if the browser cannot start or does not
    /// open its DevTools endpoint in time.
    pub fn launch(exe: &Path) -> Result<Self> {
        let profile = tempfile::Builder::new()
            .prefix("convertapp-browser-")
            .tempdir()
            .map_err(|e| ConvertError::io(std::env::temp_dir(), e))?;

        let mut command = Command::new(exe);
        command
            .arg("--headless")
            .arg("--disable-gpu")
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg("--disable-extensions")
            .arg("--disable-sync")
            .arg("--hide-scrollbars")
            .arg("--mute-audio")
            // Port 0: the browser picks a free port and writes it to
            // `DevToolsActivePort` in the profile folder.
            .arg("--remote-debugging-port=0")
            .arg(format!("--user-data-dir={}", profile.path().display()))
            .arg("about:blank")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        crate::tools::hide_console_window(&mut command);

        let mut child = command
            .spawn()
            .map_err(|e| failed(format!("cannot start {}: {e}", exe.display())))?;

        let cdp = match wait_for_devtools(&mut child, profile.path()) {
            Ok((port, path)) => Cdp::connect(port, &path).map_err(cdp_error),
            Err(e) => Err(e),
        };
        match cdp {
            Ok(cdp) => Ok(Self {
                child,
                cdp,
                _profile: profile,
            }),
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                Err(e)
            }
        }
    }

    /// Loads `url` and prints it to PDF.
    pub fn print_pdf(&mut self, url: &Url, settings: &PdfSettings) -> Result<Vec<u8>> {
        let page = self.open_page()?;
        self.load(&page, url)?;
        let result = self
            .cdp
            .call(
                Some(&page),
                "Page.printToPDF",
                json!({
                    "landscape": settings.landscape,
                    "printBackground": settings.print_background,
                    "paperWidth": settings.paper_width,
                    "paperHeight": settings.paper_height,
                    "marginTop": settings.margin,
                    "marginBottom": settings.margin,
                    "marginLeft": settings.margin,
                    "marginRight": settings.margin,
                    "preferCSSPageSize": false,
                    "generateDocumentOutline": true,
                }),
                RENDER_TIMEOUT,
            )
            .map_err(cdp_error)?;
        decode_data(&result)
    }

    /// Loads `url` in a window `width` CSS pixels wide and captures the
    /// whole page as PNG (up to [`MAX_SCREENSHOT_HEIGHT`] pixels tall).
    pub fn screenshot(&mut self, url: &Url, width: u32) -> Result<Vec<u8>> {
        let page = self.open_page()?;
        self.set_viewport(&page, width, 800)?;
        self.load(&page, url)?;

        let metrics = self.call(&page, "Page.getLayoutMetrics", json!({}))?;
        let content_height = metrics["cssContentSize"]["height"]
            .as_f64()
            .unwrap_or(800.0);
        let height = (content_height.ceil() as u32).clamp(1, MAX_SCREENSHOT_HEIGHT);
        self.set_viewport(&page, width, height)?;

        let result = self
            .cdp
            .call(
                Some(&page),
                "Page.captureScreenshot",
                json!({ "format": "png", "captureBeyondViewport": true, "fromSurface": true }),
                RENDER_TIMEOUT,
            )
            .map_err(cdp_error)?;
        decode_data(&result)
    }

    /// Opens a new blank tab and returns its flattened session id.
    fn open_page(&mut self) -> Result<String> {
        let target = self.call_browser("Target.createTarget", json!({ "url": "about:blank" }))?;
        let target_id = target["targetId"].as_str().unwrap_or_default().to_owned();
        let attached = self.call_browser(
            "Target.attachToTarget",
            json!({ "targetId": target_id, "flatten": true }),
        )?;
        let session = attached["sessionId"]
            .as_str()
            .ok_or_else(|| failed("no session id from Target.attachToTarget".into()))?
            .to_owned();
        self.call(&session, "Page.enable", json!({}))?;
        Ok(session)
    }

    /// Navigates and waits until the page, its fonts and images are loaded.
    fn load(&mut self, page: &str, url: &Url) -> Result<()> {
        let navigation = self.call(page, "Page.navigate", json!({ "url": url.as_str() }))?;
        if let Some(error) = navigation["errorText"].as_str().filter(|e| !e.is_empty()) {
            return Err(failed(format!("cannot open {url}: {error}")));
        }
        self.cdp
            .wait_event(page, "Page.loadEventFired", LOAD_TIMEOUT)
            .map_err(cdp_error)?;
        // Web fonts may still be loading after the load event.
        self.cdp
            .call(
                Some(page),
                "Runtime.evaluate",
                json!({ "expression": "document.fonts.ready.then(() => true)", "awaitPromise": true }),
                LOAD_TIMEOUT,
            )
            .map_err(cdp_error)?;
        Ok(())
    }

    fn set_viewport(&mut self, page: &str, width: u32, height: u32) -> Result<()> {
        self.call(
            page,
            "Emulation.setDeviceMetricsOverride",
            json!({ "width": width, "height": height, "deviceScaleFactor": 1, "mobile": false }),
        )
        .map(|_| ())
    }

    fn call(&mut self, page: &str, method: &str, params: Value) -> Result<Value> {
        self.cdp
            .call(Some(page), method, params, COMMAND_TIMEOUT)
            .map_err(cdp_error)
    }

    fn call_browser(&mut self, method: &str, params: Value) -> Result<Value> {
        self.cdp
            .call(None, method, params, COMMAND_TIMEOUT)
            .map_err(cdp_error)
    }
}

impl Drop for Browser {
    /// Asks the browser to quit, then makes sure the process is gone so no
    /// headless instance is left running in the background.
    fn drop(&mut self) {
        let _ = self
            .cdp
            .call(None, "Browser.close", json!({}), Duration::from_secs(2));
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Polls `<profile>/DevToolsActivePort` (first line: port, second line:
/// WebSocket path) until the browser has written it.
fn wait_for_devtools(child: &mut Child, profile: &Path) -> Result<(u16, String)> {
    let file = profile.join("DevToolsActivePort");
    let deadline = Instant::now() + LAUNCH_TIMEOUT;
    loop {
        if let Ok(text) = fs::read_to_string(&file) {
            let mut lines = text.lines();
            if let (Some(port), Some(path)) = (lines.next(), lines.next())
                && let Ok(port) = port.trim().parse::<u16>()
            {
                return Ok((port, path.trim().to_owned()));
            }
        }
        if let Ok(Some(status)) = child.try_wait() {
            return Err(failed(format!(
                "the browser exited during start-up ({status})"
            )));
        }
        if Instant::now() >= deadline {
            return Err(failed(format!(
                "the browser did not start within {} seconds",
                LAUNCH_TIMEOUT.as_secs()
            )));
        }
        thread::sleep(Duration::from_millis(50));
    }
}

/// Decodes the base64 `data` field of a printToPDF/captureScreenshot result.
fn decode_data(result: &Value) -> Result<Vec<u8>> {
    let data = result["data"]
        .as_str()
        .ok_or_else(|| failed("the browser returned no data".into()))?;
    base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|e| failed(format!("invalid data from the browser: {e}")))
}

fn failed(message: String) -> ConvertError {
    ConvertError::External {
        tool: TOOL,
        message,
    }
}

fn cdp_error(error: CdpError) -> ConvertError {
    failed(error.to_string())
}
