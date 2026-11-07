//! Rendering web pages with an installed Chromium browser (Edge or Chrome).
//!
//! The browser is started headless with a throw-away profile and driven over
//! the [Chrome DevTools Protocol](https://chromedevtools.github.io/devtools-protocol/)
//! (CDP). A small synchronous CDP client is used instead of an async crate,
//! because the rest of the engine is synchronous and only a handful of
//! commands are needed: navigate, wait for load, print to PDF, screenshot.
//!
//! - [`locate`]: finds the browser executable.
//! - [`Browser`]: one running headless browser; killed on drop.

mod cdp;
mod locate;
mod session;

pub use locate::{availability, locate};
pub use session::{Browser, PdfSettings};
