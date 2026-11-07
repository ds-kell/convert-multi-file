//! Finding and running external command-line programs (Pandoc, Typst, …).
//!
//! Converters that rely on an external program describe it with a [`Tool`].
//! The program is looked up, in order:
//! 1. an environment variable override (e.g. `CONVERTAPP_PANDOC=D:\pandoc.exe`),
//! 2. the `PATH`,
//! 3. well-known install folders (per-user installers often skip `PATH`, or
//!    the app was started before `PATH` was updated).
//!
//! Lookup only touches the file system (no process is started), so it is cheap
//! enough to repeat whenever the UI asks which formats are available. That
//! also means a tool installed while the app is running is picked up at once.

use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::{Availability, ConvertError, Result};

/// An external program a converter depends on.
#[derive(Debug, Clone, Copy)]
pub struct Tool {
    /// Display name, used in messages ("Pandoc").
    pub name: &'static str,
    /// Executable file name without extension ("pandoc").
    pub program: &'static str,
    /// Environment variable that may point at the executable.
    pub env_override: &'static str,
    /// Folders to search after `PATH`, as `(environment variable, sub-path)`
    /// pairs, e.g. `("LOCALAPPDATA", "Pandoc")`.
    pub known_dirs: &'static [(&'static str, &'static str)],
    /// How to install it, shown when it is missing.
    pub install_hint: &'static str,
}

/// [Pandoc](https://pandoc.org): universal document converter.
pub const PANDOC: Tool = Tool {
    name: "Pandoc",
    program: "pandoc",
    env_override: "CONVERTAPP_PANDOC",
    known_dirs: &[("LOCALAPPDATA", "Pandoc"), ("ProgramFiles", "Pandoc")],
    install_hint: "winget install JohnMacFarlane.Pandoc",
};

/// [Typst](https://typst.app): typesetting engine Pandoc uses to make PDFs.
pub const TYPST: Tool = Tool {
    name: "Typst",
    program: "typst",
    env_override: "CONVERTAPP_TYPST",
    known_dirs: &[("LOCALAPPDATA", r"Microsoft\WinGet\Links")],
    install_hint: "winget install Typst.Typst",
};

impl Tool {
    /// Full path of the executable, or `None` if it cannot be found.
    pub fn locate(&self) -> Option<PathBuf> {
        if let Some(path) = std::env::var_os(self.env_override).map(PathBuf::from) {
            // An explicit override is authoritative, even if it is wrong, so
            // the user sees why their setting does not work.
            return path.is_file().then_some(path);
        }

        let file_name = executable_name(self.program);
        let path_dirs = std::env::var_os("PATH")
            .map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
            .unwrap_or_default();
        let known_dirs = self.known_dirs.iter().filter_map(|(var, sub)| {
            std::env::var_os(var).map(|base| PathBuf::from(base).join(sub))
        });

        path_dirs
            .into_iter()
            .chain(known_dirs)
            .map(|dir| dir.join(&file_name))
            .find(|candidate| candidate.is_file())
    }

    /// [`Availability::Available`] if the tool can be found, otherwise a
    /// reason that tells the user how to install it.
    pub fn availability(&self) -> Availability {
        match self.locate() {
            Some(_) => Availability::Available,
            None => Availability::Unavailable {
                reason: self.missing_reason(),
            },
        }
    }

    /// Like [`locate`](Self::locate), but as a conversion error.
    pub(crate) fn require(&self) -> Result<PathBuf> {
        self.locate().ok_or_else(|| ConvertError::External {
            tool: self.name,
            message: self.missing_reason(),
        })
    }

    fn missing_reason(&self) -> String {
        format!(
            "{} is not installed. Install it with: {}",
            self.name, self.install_hint
        )
    }
}

/// `pandoc` → `pandoc.exe` on Windows.
fn executable_name(program: &str) -> String {
    if cfg!(windows) {
        format!("{program}.exe")
    } else {
        program.to_owned()
    }
}

/// Runs `exe` with `args` in `cwd` and waits for it, up to `timeout`.
///
/// - No console window flashes up on Windows.
/// - Standard output is discarded; standard error is collected and returned
///   in the error message when the program fails.
/// - A program still running after `timeout` is killed.
///
/// # Errors
/// [`ConvertError::External`] if the program cannot start, exits with an
/// error, or times out.
pub(crate) fn run<I, S>(
    tool: &Tool,
    exe: &Path,
    args: I,
    cwd: &Path,
    timeout: Duration,
) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let failure = |message: String| ConvertError::External {
        tool: tool.name,
        message,
    };

    let mut command = Command::new(exe);
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    hide_console_window(&mut command);

    tracing::debug!(?command, "running external tool");
    let mut child = command
        .spawn()
        .map_err(|e| failure(format!("cannot start {}: {e}", exe.display())))?;

    // Drain stderr on a thread: a chatty program would otherwise block once
    // the pipe buffer is full, and we would wrongly report a timeout.
    let mut stderr = child.stderr.take().expect("stderr is piped");
    let reader = thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(failure(format!(
                    "did not finish within {} seconds",
                    timeout.as_secs()
                )));
            }
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(e) => return Err(failure(e.to_string())),
        }
    };

    let stderr = reader.join().unwrap_or_default();
    if status.success() {
        if !stderr.trim().is_empty() {
            tracing::debug!(tool = tool.name, warnings = %stderr.trim(), "tool warnings");
        }
        Ok(())
    } else {
        let detail = stderr.trim();
        Err(failure(if detail.is_empty() {
            format!("exited with {status}")
        } else {
            detail.to_owned()
        }))
    }
}

/// Prevents a console window from flashing up for command (Windows only).
#[cfg(windows)]
pub(crate) fn hide_console_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    /// `CREATE_NO_WINDOW` process creation flag.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
pub(crate) fn hide_console_window(_command: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::*;

    const MISSING: Tool = Tool {
        name: "Nope",
        program: "definitely-not-installed-convertapp",
        env_override: "CONVERTAPP_TEST_NOPE",
        known_dirs: &[],
        install_hint: "install nope",
    };

    #[test]
    fn missing_tool_reports_install_hint() {
        assert!(MISSING.locate().is_none());
        match MISSING.availability() {
            Availability::Unavailable { reason } => {
                assert!(reason.contains("Nope is not installed"));
                assert!(reason.contains("install nope"));
            }
            Availability::Available => panic!("should be missing"),
        }
        assert!(matches!(
            MISSING.require(),
            Err(ConvertError::External { tool: "Nope", .. })
        ));
    }

    #[cfg(windows)]
    #[test]
    fn run_reports_failures_and_success() {
        let cmd = PathBuf::from(std::env::var_os("ComSpec").expect("ComSpec is set on Windows"));
        let dir = std::env::temp_dir();
        let timeout = Duration::from_secs(10);

        run(&MISSING, &cmd, ["/C", "exit 0"], &dir, timeout).unwrap();

        let err = run(
            &MISSING,
            &cmd,
            ["/C", "echo boom 1>&2 & exit 3"],
            &dir,
            timeout,
        )
        .unwrap_err();
        assert!(err.to_string().contains("boom"), "{err}");
    }

    #[cfg(windows)]
    #[test]
    fn run_kills_programs_that_hang() {
        let powershell =
            PathBuf::from(r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe");
        if !powershell.is_file() {
            return;
        }
        let started = Instant::now();
        let err = run(
            &MISSING,
            &powershell,
            ["-NoProfile", "-Command", "Start-Sleep -Seconds 30"],
            &std::env::temp_dir(),
            Duration::from_millis(500),
        )
        .unwrap_err();
        assert!(err.to_string().contains("did not finish"), "{err}");
        assert!(started.elapsed() < Duration::from_secs(10));
    }
}
