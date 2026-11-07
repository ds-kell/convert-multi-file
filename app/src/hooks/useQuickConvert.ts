import { useCallback, useEffect, useRef, useState } from "react";

import { convertFiles, onQuickPending, takeQuickRequests } from "../api";
import type { ConvertEvent, FileStatus, FormatInfo, Settings } from "../types";

/** Settings used for context-menu conversions (no options UI in this window). */
const QUICK_SETTINGS: Settings = { jpegQuality: 90, theme: "auto", outputDir: null };

/** One file in the quick-convert window. */
export interface QuickRow {
  /** Unique across batches: `${batch}:${index}`. */
  key: string;
  path: string;
  name: string;
  target: FormatInfo;
  status: FileStatus;
}

/** File name from a Windows or POSIX path. */
function baseName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

/**
 * Drives the quick-convert window: fetches batches queued by the Explorer
 * context menu and converts them one after another. Requests that arrive
 * while a batch is running are picked up as soon as it finishes.
 */
export function useQuickConvert() {
  const [rows, setRows] = useState<QuickRow[]>([]);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Refs instead of state: read and written from async code, never rendered.
  const runningRef = useRef(false);
  const pendingRef = useRef(false);
  const batchCounter = useRef(0);

  /** Converts everything queued, looping while new requests keep arriving. */
  const drain = useCallback(async () => {
    if (runningRef.current) {
      pendingRef.current = true; // picked up by the loop below
      return;
    }
    runningRef.current = true;
    setRunning(true);
    try {
      do {
        pendingRef.current = false;
        for (const batch of await takeQuickRequests()) {
          await runBatch(batch.target, batch.paths);
        }
      } while (pendingRef.current);
    } catch (e) {
      setError(String(e));
    } finally {
      runningRef.current = false;
      setRunning(false);
    }
  }, []);

  /** Adds rows for one batch and converts it, updating rows from events. */
  const runBatch = async (target: FormatInfo, paths: string[]) => {
    const batch = batchCounter.current++;
    const keys = paths.map((_, index) => `${batch}:${index}`);
    setRows((current) => [
      ...current,
      ...paths.map((path, index) => ({
        key: keys[index]!,
        path,
        name: baseName(path),
        target,
        status: { kind: "queued" } as FileStatus,
      })),
    ]);

    const setStatus = (index: number, status: FileStatus) => {
      const key = keys[index];
      setRows((current) => current.map((row) => (row.key === key ? { ...row, status } : row)));
    };

    const onEvent = (event: ConvertEvent) => {
      switch (event.event) {
        case "started":
          setStatus(event.data.index, { kind: "converting", fraction: 0 });
          break;
        case "progress":
          setStatus(event.data.index, { kind: "converting", fraction: event.data.fraction });
          break;
        case "done":
          setStatus(event.data.index, { kind: "done", output: event.data.output });
          break;
        case "skipped":
          setStatus(event.data.index, { kind: "skipped", reason: event.data.reason });
          break;
        case "failed":
          setStatus(event.data.index, { kind: "failed", message: event.data.message });
          break;
        case "finished":
          break; // totals are derived from the rows
      }
    };

    await convertFiles(paths, target.id, QUICK_SETTINGS, onEvent);
  };

  // Subscribe first, then take what is already queued, so nothing is missed.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    onQuickPending(() => void drain()).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
      void drain();
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [drain]);

  const count = (kind: FileStatus["kind"]) => rows.filter((r) => r.status.kind === kind).length;

  return {
    rows,
    running,
    error,
    done: count("done"),
    failed: count("failed"),
    skipped: count("skipped"),
  };
}
