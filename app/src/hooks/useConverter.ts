import { useCallback, useRef, useState } from "react";

import { convertFiles, inspectFiles } from "../api";
import type { ConvertEvent, FileItem, FileStatus, FormatInfo, LockedTarget, Settings, Summary } from "../types";

const IDLE: FileStatus = { kind: "idle" };

/** Everything the UI needs to manage the file list and run conversions. */
export interface Converter {
  files: FileItem[];
  /** Targets all recognised files can be converted to. */
  targets: FormatInfo[];
  /** Formats all files can be merged into (one output). */
  mergeTargets: FormatInfo[];
  /** Targets that need a missing program (shown greyed out). */
  locked: LockedTarget[];
  /** `true` while `inspect_files` or `convert_files` is running. */
  busy: boolean;
  /** Totals of the last finished batch, cleared when the list changes. */
  summary: Summary | null;
  /** Unexpected error (backend crashed, IPC failure, …). */
  error: string | null;
  addFiles: (paths: string[]) => Promise<void>;
  removeFile: (path: string) => Promise<void>;
  clear: () => void;
  convert: (target: string, settings: Settings) => Promise<void>;
}

/**
 * State machine behind the main screen: the file list, its inspection results
 * and the progress of a running conversion.
 */
export function useConverter(): Converter {
  const [files, setFiles] = useState<FileItem[]>([]);
  const [targets, setTargets] = useState<FormatInfo[]>([]);
  const [mergeTargets, setMergeTargets] = useState<FormatInfo[]>([]);
  const [locked, setLocked] = useState<LockedTarget[]>([]);
  const [busy, setBusy] = useState(false);
  const [summary, setSummary] = useState<Summary | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Latest list, readable from async callbacks without stale closures.
  const filesRef = useRef(files);
  filesRef.current = files;

  /** Re-inspects `paths` and replaces the list with fresh, idle rows. */
  const inspect = useCallback(async (paths: string[]) => {
    setSummary(null);
    setError(null);
    if (paths.length === 0) {
      setFiles([]);
      setTargets([]);
      setMergeTargets([]);
      setLocked([]);
      return;
    }
    setBusy(true);
    try {
      const result = await inspectFiles(paths);
      setFiles(result.files.map((file) => ({ ...file, status: IDLE })));
      setTargets(result.targets);
      setMergeTargets(result.mergeTargets);
      setLocked(result.locked);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }, []);

  const addFiles = useCallback(
    async (paths: string[]) => {
      // Keep existing order, append new files, ignore duplicates.
      const known = new Set(filesRef.current.map((f) => f.path));
      const merged = [...filesRef.current.map((f) => f.path), ...paths.filter((p) => !known.has(p))];
      await inspect(merged);
    },
    [inspect],
  );

  const removeFile = useCallback(
    async (path: string) => {
      await inspect(filesRef.current.map((f) => f.path).filter((p) => p !== path));
    },
    [inspect],
  );

  const clear = useCallback(() => {
    setFiles([]);
    setTargets([]);
    setMergeTargets([]);
    setLocked([]);
    setSummary(null);
    setError(null);
  }, []);

  const convert = useCallback(async (target: string, settings: Settings) => {
    // Only files without an inspection error are sent to the backend.
    const batch = filesRef.current.filter((f) => f.error === null);
    if (batch.length === 0) return;

    const batchPaths = batch.map((f) => f.path);
    const queued = new Set(batchPaths);

    /** Updates the row of the file at `index` in the batch. */
    const setStatus = (index: number, status: FileStatus) => {
      const path = batchPaths[index];
      setFiles((rows) => rows.map((row) => (row.path === path ? { ...row, status } : row)));
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
          setStatus(event.data.index, { kind: "skipped", format: event.data.format });
          break;
        case "failed":
          setStatus(event.data.index, { kind: "failed", message: event.data.message });
          break;
        case "finished":
          setSummary(event.data);
          break;
      }
    };

    setBusy(true);
    setSummary(null);
    setError(null);
    setFiles((rows) => rows.map((row) => (queued.has(row.path) ? { ...row, status: { kind: "queued" } } : row)));
    try {
      await convertFiles(batchPaths, target, settings, onEvent);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }, []);

  return { files, targets, mergeTargets, locked, busy, summary, error, addFiles, removeFile, clear, convert };
}
