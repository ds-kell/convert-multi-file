// Types shared with the Rust backend. They mirror `src-tauri/src/dto.rs`;
// keep both files in sync when changing either.

/** A file format as described by the backend. */
export interface FormatInfo {
  /** Canonical extension, also used as identifier ("jpg", "html", …). */
  id: string;
  /** Display name ("JPEG", "Markdown", …). */
  name: string;
  category: "image" | "document" | "other";
}

/** One inspected input file. */
export interface FileInfo {
  path: string;
  name: string;
  format: FormatInfo | null;
  /** Why this file cannot be converted, if it cannot. */
  error: string | null;
}

/** Result of the `inspect_files` command. */
export interface Inspection {
  files: FileInfo[];
  /** Targets every recognised file can be converted to. */
  targets: FormatInfo[];
}

export type Theme = "auto" | "light" | "dark" | "none";

/** Settings sent with `convert_files`. */
export interface Settings {
  jpegQuality: number;
  theme: Theme;
  /** `null` = save next to each input file. */
  outputDir: string | null;
}

/** Events streamed by `convert_files`. `index` refers to the `paths` array sent. */
export type ConvertEvent =
  | { event: "started"; data: { index: number } }
  | { event: "progress"; data: { index: number; fraction: number } }
  | { event: "done"; data: { index: number; output: string } }
  | { event: "skipped"; data: { index: number; reason: string } }
  | { event: "failed"; data: { index: number; message: string } }
  | { event: "finished"; data: { converted: number; skipped: number; failed: number } };

/** Where a file is in the conversion lifecycle (UI only). */
export type FileStatus =
  | { kind: "idle" }
  | { kind: "queued" }
  | { kind: "converting"; fraction: number }
  | { kind: "done"; output: string }
  | { kind: "skipped"; reason: string }
  | { kind: "failed"; message: string };

/** A row in the file list. */
export interface FileItem extends FileInfo {
  status: FileStatus;
}

export type Summary = Extract<ConvertEvent, { event: "finished" }>["data"];
