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
  error: FileIssue | null;
}

/** Why a file cannot be converted (mirrors `FileIssue` in Rust); translated by the UI. */
export type FileIssue =
  | { kind: "unknownFormat" }
  | { kind: "unsupported" }
  | { kind: "needsTool"; alternatives: Requirement[][] }
  | { kind: "unreadable"; detail: string };

/** Result of the `inspect_files` command. */
export interface Inspection {
  files: FileInfo[];
  /** Targets every recognised file can be converted to. */
  targets: FormatInfo[];
  /** Formats all files can be merged into, as one output. */
  mergeTargets: FormatInfo[];
  /** Targets that need a program that is not installed. */
  locked: LockedTarget[];
}

/** A program the user can install (mirrors `RequirementInfo` in Rust). */
export interface Requirement {
  name: string;
  /** Official download page. */
  url: string;
  /** One-line install command for the current OS (`winget …`, `brew …`, `sudo apt …`). */
  installCommand: string | null;
  /** `false` for paid software. */
  free: boolean;
}

/** A target shown greyed out, with how to unlock it. */
export interface LockedTarget {
  format: FormatInfo;
  /** Install all programs of any one inner list. Free options first. */
  alternatives: Requirement[][];
}

export type Theme = "auto" | "light" | "dark" | "none";

/** Image resizing (mirrors `ResizeSetting` in Rust). */
export type ResizeSetting =
  | { mode: "original" }
  | { mode: "fit"; maxWidth: number | null; maxHeight: number | null }
  | { mode: "percent"; percent: number };

export type PdfPage = "a4" | "image";

/** How images are joined into one picture (mirrors `StitchSetting` in Rust). */
export interface StitchSetting {
  direction: "vertical" | "horizontal";
  /** Pixels of empty space between images. */
  gap: number;
  /** Shrink images to a common width/height. */
  sameSize: boolean;
}

export type Paper = "a4" | "a3" | "a5" | "letter" | "legal";

/** Office suite for Word/Excel/PowerPoint files (mirrors `Engine` in Rust). */
export type OfficeEngine = "auto" | "microsoft" | "libreoffice";

/** Page setup for HTML/Markdown → PDF/PNG (mirrors `WebSetting` in Rust). */
export interface WebSetting {
  paper: Paper;
  landscape: boolean;
  /** Margin on every side, in millimetres (0–100). */
  marginMm: number;
  printBackground: boolean;
  /** Screenshot width in CSS pixels (320–4096). */
  viewportWidth: number;
}

/** Settings sent with `convert_files`. */
export interface Settings {
  /** Lossy quality 1–100 (JPEG, WebP, photos in PDF). */
  quality: number;
  webpLossless: boolean;
  resize: ResizeSetting;
  pdfPage: PdfPage;
  /** Join all files into one output instead of one output per file. */
  merge: boolean;
  stitch: StitchSetting;
  theme: Theme;
  /** Word file whose styles are applied to generated .docx files. */
  referenceDocx: string | null;
  web: WebSetting;
  officeEngine: OfficeEngine;
  /** `null` = save next to each input file. */
  outputDir: string | null;
}

/** Settings used when the user has not changed anything. */
export const DEFAULT_SETTINGS: Settings = {
  quality: 90,
  webpLossless: false,
  resize: { mode: "original" },
  pdfPage: "a4",
  merge: false,
  stitch: { direction: "vertical", gap: 0, sameSize: true },
  theme: "auto",
  referenceDocx: null,
  web: { paper: "a4", landscape: false, marginMm: 10, printBackground: true, viewportWidth: 1280 },
  officeEngine: "auto",
  outputDir: null,
};

/** Events streamed by `convert_files`. `index` refers to the `paths` array sent. */
export type ConvertEvent =
  | { event: "started"; data: { index: number } }
  | { event: "progress"; data: { index: number; fraction: number } }
  | { event: "done"; data: { index: number; output: string } }
  | { event: "skipped"; data: { index: number; format: string } }
  | { event: "failed"; data: { index: number; message: string } }
  | { event: "finished"; data: { converted: number; skipped: number; failed: number } };

/** Where a file is in the conversion lifecycle (UI only). */
export type FileStatus =
  | { kind: "idle" }
  | { kind: "queued" }
  | { kind: "converting"; fraction: number }
  | { kind: "done"; output: string }
  | { kind: "skipped"; format: string }
  | { kind: "failed"; message: string };

/** A row in the file list. */
export interface FileItem extends FileInfo {
  status: FileStatus;
}

export type Summary = Extract<ConvertEvent, { event: "finished" }>["data"];

/** Files from the Explorer context menu, grouped by target (`take_quick_requests`). */
export interface QuickBatch {
  target: FormatInfo;
  paths: string[];
}
