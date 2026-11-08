// Thin, typed wrappers around Tauri commands and plugins. Components never
// call `invoke` directly, so the IPC surface lives in one place.

import { Channel, invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";

import type { Language } from "./i18n";
import type { ConvertEvent, Inspection, QuickBatch, Settings } from "./types";

/** `false` when the UI runs in a plain browser (e.g. `npm run dev` alone). */
export const insideTauri = isTauri();

/** Label of the window this UI runs in: "main", "quick", or "browser" outside Tauri. */
export const windowLabel: string = insideTauri ? getCurrentWindow().label : "browser";

/** Closes the current window. */
export function closeWindow(): Promise<void> {
  return getCurrentWindow().close();
}

// ---------- Quick convert (Explorer context menu) ----------

/** Waits until context-menu requests stop arriving, then returns them as batches. */
export function takeQuickRequests(): Promise<QuickBatch[]> {
  return invoke<QuickBatch[]>("take_quick_requests");
}

/** Calls `handler` whenever new context-menu requests are queued. */
export function onQuickPending(handler: () => void): Promise<UnlistenFn> {
  return listen("quick://pending", handler);
}

// ---------- Explorer context menu integration ----------

/** Whether the "Convert to" entry is in the Explorer context menu. */
export function contextMenuEnabled(): Promise<boolean> {
  return invoke<boolean>("context_menu_enabled");
}

/**
 * Adds or removes the Explorer context-menu entry, titled in `language`
 * ("Convert to" / "Chuyển sang"); resolves to the new state.
 */
export function setContextMenu(enabled: boolean, language: Language): Promise<boolean> {
  return invoke<boolean>("set_context_menu", { enabled, language });
}

/** Sets the title of the current window. */
export function setWindowTitle(title: string): Promise<void> {
  return getCurrentWindow().setTitle(title);
}

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

// Dialog titles are passed in by the caller, already translated.

/** Opens the native file picker. Returns the chosen paths (empty if cancelled). */
export async function pickFiles(title: string): Promise<string[]> {
  const selected = await open({ multiple: true, directory: false, title });
  if (selected === null) return [];
  return Array.isArray(selected) ? selected : [selected];
}

/** Lets the user pick a Word file to use as a style template. */
export async function pickWordTemplate(title: string): Promise<string | null> {
  const selected = await open({
    multiple: false,
    directory: false,
    title,
    filters: [{ name: "Word", extensions: ["docx"] }],
  });
  return typeof selected === "string" ? selected : null;
}

/** Opens the native folder picker. Returns `null` if cancelled. */
export async function pickFolder(title: string): Promise<string | null> {
  const selected = await open({ directory: true, multiple: false, title });
  return typeof selected === "string" ? selected : null;
}

/** Shows `path` selected in Explorer / Finder. */
export function revealInFolder(path: string): Promise<void> {
  return revealItemInDir(path);
}

/** Opens an https page (e.g. a download page) in the default browser. */
export function openInBrowser(url: string): Promise<void> {
  return openUrl(url);
}

/** Copies `text` to the clipboard. */
export function copyText(text: string): Promise<void> {
  return navigator.clipboard.writeText(text);
}
