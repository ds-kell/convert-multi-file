import { useEffect, useState } from "react";

import { insideTauri, pickFiles, pickFolder, revealInFolder } from "./api";
import { ContextMenuToggle } from "./components/ContextMenuToggle";
import { DropZone } from "./components/DropZone";
import { FileList } from "./components/FileList";
import { SettingsPanel } from "./components/SettingsPanel";
import { TargetPicker } from "./components/TargetPicker";
import { useConverter } from "./hooks/useConverter";
import { useFileDrop } from "./hooks/useFileDrop";
import type { Settings } from "./types";

const DEFAULT_SETTINGS: Settings = { jpegQuality: 90, theme: "auto", outputDir: null };

/** Main (and only) screen: add files → pick a target → convert. */
export default function App() {
  const converter = useConverter();
  const { files, targets, busy, summary, error } = converter;

  const [target, setTarget] = useState<string | null>(null);
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);

  const hovering = useFileDrop((paths) => void converter.addFiles(paths));

  // Drop the selected target when it is no longer valid for the file list.
  useEffect(() => {
    if (target !== null && !targets.some((t) => t.id === target)) setTarget(null);
  }, [targets, target]);

  const convertible = files.filter((f) => f.error === null).length;
  const canConvert = !busy && target !== null && convertible > 0;

  const handlePickFiles = async () => {
    const paths = await pickFiles();
    if (paths.length > 0) await converter.addFiles(paths);
  };

  const handlePickFolder = async () => {
    const folder = await pickFolder();
    if (folder !== null) setSettings((s) => ({ ...s, outputDir: folder }));
  };

  const handleConvert = () => {
    if (target !== null) void converter.convert(target, settings);
  };

  return (
    <main className="app">
      <header className="app__header">
        <h1>ConvertApp</h1>
        <div className="app__header-actions">
          {files.length > 0 && (
            <button type="button" className="link-button" onClick={converter.clear} disabled={busy}>
              Xoá danh sách
            </button>
          )}
          <ContextMenuToggle />
        </div>
      </header>

      {!insideTauri && (
        <p className="banner">Đang chạy trong trình duyệt: hãy mở bằng <code>cargo tauri dev</code> để chuyển đổi file.</p>
      )}

      <DropZone hovering={hovering} compact={files.length > 0} disabled={busy} onPick={handlePickFiles} />

      {files.length > 0 && (
        <>
          <FileList files={files} busy={busy} onRemove={(p) => void converter.removeFile(p)} onReveal={(p) => void revealInFolder(p)} />

          <section className="panel">
            <h2 className="panel__title">Chuyển sang</h2>
            <TargetPicker targets={targets} selected={target} disabled={busy} onSelect={setTarget} />
            {targets.length > 0 && (
              <SettingsPanel
                target={target}
                settings={settings}
                disabled={busy}
                onChange={setSettings}
                onPickFolder={handlePickFolder}
              />
            )}
          </section>
        </>
      )}

      {error !== null && <p className="banner banner--error">{error}</p>}

      {files.length > 0 && (
        <footer className="app__footer">
          <span className="summary" aria-live="polite">
            {summary !== null
              ? `Xong: ${summary.converted} thành công` +
                (summary.skipped > 0 ? `, ${summary.skipped} bỏ qua` : "") +
                (summary.failed > 0 ? `, ${summary.failed} lỗi` : "")
              : `${convertible}/${files.length} file có thể chuyển`}
          </span>
          <button type="button" className="button button--primary" onClick={handleConvert} disabled={!canConvert}>
            {busy ? "Đang chuyển…" : "Chuyển đổi"}
          </button>
        </footer>
      )}
    </main>
  );
}
