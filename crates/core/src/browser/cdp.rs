//! Minimal synchronous Chrome DevTools Protocol client over a WebSocket.
//!
//! CDP is JSON-RPC-like: every command carries an `id` and gets a reply with
//! the same `id`; in between, the browser pushes events (`method` + `params`).
//! With "flattened" sessions, page-level messages carry a `sessionId`.
//!
//! Events that arrive while waiting for a reply are buffered so that a later
//! [`Cdp::wait_event`] still sees them.

use std::collections::VecDeque;
use std::io;
use std::net::TcpStream;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tungstenite::protocol::WebSocketConfig;
use tungstenite::{Message, WebSocket};

/// Errors from the protocol layer; turned into user-facing errors by the
/// caller.
#[derive(Debug, thiserror::Error)]
pub(crate) enum CdpError {
    /// The browser did not answer in time.
    #[error("the browser did not respond within {0} seconds")]
    Timeout(u64),
    /// The browser answered with an error object.
    #[error("{method} failed: {message}")]
    Command {
        /// The CDP method that failed.
        method: String,
        /// The browser's error message.
        message: String,
    },
    /// The connection failed or closed.
    #[error("connection to the browser failed: {0}")]
    Connection(String),
}

/// Largest message accepted: printToPDF returns the whole PDF base64-encoded
/// in one message.
const MAX_MESSAGE: usize = 512 * 1024 * 1024;

/// A connection to the browser's DevTools endpoint.
pub(crate) struct Cdp {
    socket: WebSocket<TcpStream>,
    next_id: u64,
    /// Events received while waiting for something else.
    pending_events: VecDeque<Value>,
}

impl std::fmt::Debug for Cdp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cdp")
            .field("next_id", &self.next_id)
            .finish_non_exhaustive()
    }
}

impl Cdp {
    /// Connects to `ws://127.0.0.1:{port}{path}`.
    pub(crate) fn connect(port: u16, path: &str) -> Result<Self, CdpError> {
        let stream = TcpStream::connect(("127.0.0.1", port)).map_err(connection)?;
        stream.set_nodelay(true).map_err(connection)?;
        let config = WebSocketConfig::default()
            .max_message_size(Some(MAX_MESSAGE))
            .max_frame_size(Some(MAX_MESSAGE));
        let url = format!("ws://127.0.0.1:{port}{path}");
        let (socket, _response) =
            tungstenite::client::client_with_config(url, stream, Some(config))
                .map_err(|e| CdpError::Connection(e.to_string()))?;
        Ok(Self {
            socket,
            next_id: 1,
            pending_events: VecDeque::new(),
        })
    }

    /// Sends `method` (to `session` if given) and waits for its result.
    pub(crate) fn call(
        &mut self,
        session: Option<&str>,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, CdpError> {
        let id = self.next_id;
        self.next_id += 1;

        let mut message = json!({ "id": id, "method": method, "params": params });
        if let Some(session) = session {
            message["sessionId"] = Value::from(session);
        }
        self.socket
            .send(Message::text(message.to_string()))
            .map_err(|e| CdpError::Connection(e.to_string()))?;

        let deadline = Instant::now() + timeout;
        loop {
            let mut reply = self.read(deadline, timeout)?;
            if reply.get("id").and_then(Value::as_u64) == Some(id) {
                if let Some(error) = reply.get("error") {
                    return Err(CdpError::Command {
                        method: method.to_owned(),
                        message: error["message"]
                            .as_str()
                            .unwrap_or("unknown error")
                            .to_owned(),
                    });
                }
                return Ok(reply["result"].take());
            }
            if reply.get("method").is_some() {
                self.pending_events.push_back(reply);
            }
        }
    }

    /// Waits for the event `method` on `session` and returns its `params`.
    pub(crate) fn wait_event(
        &mut self,
        session: &str,
        method: &str,
        timeout: Duration,
    ) -> Result<Value, CdpError> {
        let matches = |event: &Value| {
            event["method"] == method && event["sessionId"].as_str() == Some(session)
        };

        if let Some(index) = self.pending_events.iter().position(matches) {
            let mut event = self.pending_events.remove(index).expect("index is valid");
            return Ok(event["params"].take());
        }

        let deadline = Instant::now() + timeout;
        loop {
            let mut message = self.read(deadline, timeout)?;
            if matches(&message) {
                return Ok(message["params"].take());
            }
            // Other events are not needed later; replies to commands cannot
            // arrive here because calls wait for their own reply.
        }
    }

    /// Reads the next JSON message, giving up at `deadline`.
    fn read(&mut self, deadline: Instant, timeout: Duration) -> Result<Value, CdpError> {
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(CdpError::Timeout(timeout.as_secs()));
            }
            self.socket
                .get_ref()
                .set_read_timeout(Some(remaining))
                .map_err(connection)?;

            match self.socket.read() {
                Ok(Message::Text(text)) => {
                    return serde_json::from_str(text.as_str())
                        .map_err(|e| CdpError::Connection(format!("invalid JSON: {e}")));
                }
                Ok(Message::Close(_)) => {
                    return Err(CdpError::Connection(
                        "the browser closed the connection".into(),
                    ));
                }
                Ok(_) => {} // ping/pong/binary: not used by CDP
                Err(tungstenite::Error::Io(e))
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) =>
                {
                    return Err(CdpError::Timeout(timeout.as_secs()));
                }
                Err(e) => return Err(CdpError::Connection(e.to_string())),
            }
        }
    }
}

fn connection(error: io::Error) -> CdpError {
    CdpError::Connection(error.to_string())
}
