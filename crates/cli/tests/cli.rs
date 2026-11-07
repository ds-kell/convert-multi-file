//! Runs the real `convertapp-cli` binary and checks its output and exit codes.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Path of the binary built by Cargo for this test run.
const BIN: &str = env!("CARGO_BIN_EXE_convertapp-cli");

fn run(args: &[&str]) -> Output {
    Command::new(BIN)
        .args(args)
        .output()
        .expect("failed to start convertapp-cli")
}

/// Writes a tiny PNG into `dir` and returns its path.
fn sample_png(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    image::RgbaImage::from_pixel(4, 4, image::Rgba([10, 20, 30, 255]))
        .save(&path)
        .unwrap();
    path
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn formats_lists_targets() {
    let dir = tempfile::tempdir().unwrap();
    let png = sample_png(dir.path(), "a.png");

    let output = run(&["formats", png.to_str().unwrap()]);

    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("(PNG) ->"), "{text}");
    assert!(text.contains("jpg"), "{text}");
}

#[test]
fn convert_writes_output_and_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let png = sample_png(dir.path(), "a.png");
    let out_dir = dir.path().join("out");

    let output = run(&[
        "convert",
        "--to",
        "jpg",
        "--out",
        out_dir.to_str().unwrap(),
        png.to_str().unwrap(),
    ]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(out_dir.join("a.jpg").is_file());
    assert!(stdout(&output).contains("[ok]"));
}

#[test]
fn convert_continues_after_a_failure_and_exits_with_1() {
    let dir = tempfile::tempdir().unwrap();
    let good = sample_png(dir.path(), "good.png");
    let missing = dir.path().join("missing.png");

    let output = run(&[
        "convert",
        "--to",
        "bmp",
        missing.to_str().unwrap(),
        good.to_str().unwrap(),
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("[error]"));
    assert!(
        dir.path().join("good.bmp").is_file(),
        "good file must still be converted"
    );
    assert!(stdout(&output).contains("1 converted, 1 failed"));
}

#[test]
fn markdown_converts_with_theme_option() {
    let dir = tempfile::tempdir().unwrap();
    let md = dir.path().join("notes.md");
    std::fs::write(&md, "# Ghi chú\n\nXin chào").unwrap();

    let output = run(&[
        "convert",
        "--to",
        "html",
        "--theme",
        "dark",
        md.to_str().unwrap(),
    ]);

    assert!(output.status.success(), "{}", stderr(&output));
    let html = std::fs::read_to_string(dir.path().join("notes.html")).unwrap();
    assert!(html.contains("data-theme=\"dark\""));
    assert!(html.contains("<title>Ghi chú</title>"));
}

#[test]
fn unknown_target_format_is_a_usage_error() {
    let output = run(&["convert", "--to", "xyz", "a.png"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("unknown format 'xyz'"));
}
