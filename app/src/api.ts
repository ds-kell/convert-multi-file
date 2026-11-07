// Thin, typed wrappers around Tauri commands and plugins. Components never
// call `invoke` directly, so the IPC surface lives in one place.

import { Channel, invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

import type { ConvertEvent, Inspection, Settings } from "./types";

/** `false` when the UI runs in a plain browser (e.g. `npm run dev` alone). */
export const insideTauri = isTauri();

/** Detects formats and the common conversion targets for `paths`. */
export function inspectFiles(paths: string[]): Promise<Inspection> {
  return invoke<Inspection>("inspect_files", { paths });
}

/**
 * Converts `paths` to `target`. Resolves when every file has been processed;
 * progress arrives through `onEvent` in the meantime.
 */
export function convertFiles(
  paths: string[],
  target: string,
  settings: Settings,
  onEvent: (event: ConvertEvent) => void,
): Promise<void> {
  const channel = new Channel<ConvertEvent>();
  channel.onmessage = onEvent;
  return invoke<void>("convert_files", { paths, target, settings, onEvent: channel });
}

/** Opens the native file picker. Returns the chosen paths (empty if cancelled). */
export async function pickFiles(): Promise<string[]> {
  const selected = await open({ multiple: true, directory: false, title: "Chọn file cần chuyển đổi" });
  if (selected === null) return [];
  return Array.isArray(selected) ? selected : [selected];
}

/** Opens the native folder picker. Returns `null` if cancelled. */
export async function pickFolder(): Promise<string | null> {
  const selected = await open({ directory: true, multiple: false, title: "Chọn thư mục lưu kết quả" });
  return typeof selected === "string" ? selected : null;
}

/** Shows `path` selected in Explorer / Finder. */
export function revealInFolder(path: string): Promise<void> {
  return revealItemInDir(path);
}
