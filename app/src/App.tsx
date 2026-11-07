import { useEffect, useState } from "react";

import { insideTauri, pickFiles, pickFolder, pickWordTemplate, revealInFolder } from "./api";
import { ContextMenuToggle } from "./components/ContextMenuToggle";
import { DropZone } from "./components/DropZone";
import { FileList } from "./components/FileList";
import { LanguagePicker } from "./components/LanguagePicker";
import { SettingsPanel } from "./components/SettingsPanel";
import { TargetPicker } from "./components/TargetPicker";
import { useConverter } from "./hooks/useConverter";
import { useFileDrop } from "./hooks/useFileDrop";
import { useI18n } from "./i18n";
import { DEFAULT_SETTINGS, type Settings } from "./types";

/** Formats opened by an office suite (Word/Excel/PowerPoint or LibreOffice). */
const OFFICE_FORMATS = new Set(["docx", "doc", "odt", "rtf", "xlsx", "xls", "ods", "csv", "pptx", "ppt", "odp"]);

/** Main (and only) screen: add files → pick a target → convert. */
export default function App() {
  const { t, tn } = useI18n();
  const converter = useConverter();
  const { files, targets, busy, summary, error } = converter;

  const [target, setTarget] = useState<string | null>(null);
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);

  const hovering = useFileDrop((paths) => void converter.addFiles(paths));

  // Drop the selected target when it is no longer valid for the file list.
  useEffect(() => {
    if (target !== null && !targets.some((t) => t.id === target)) setTarget(null);
  }, [targets, target]);

  const ready = files.filter((f) => f.error === null);
  const convertible = ready.length;
  const imagesOnly = convertible > 0 && ready.every((f) => f.format?.category === "image");
  const webPagesOnly = convertible > 0 && ready.every((f) => f.format?.id === "html" || f.format?.id === "md");
  const officeOnly = convertible > 0 && ready.every((f) => OFFICE_FORMATS.has(f.format?.id ?? ""));
  const canConvert = !busy && target !== null && convertible > 0;

  const handlePickFiles = async () => {
    const paths = await pickFiles(t("dialog.pickFiles"));
    if (paths.length > 0) await converter.addFiles(paths);
  };

  const handlePickFolder = async () => {
    const folder = await pickFolder(t("dialog.pickFolder"));
    if (folder !== null) setSettings((s) => ({ ...s, outputDir: folder }));
  };

  const handlePickWordTemplate = async () => {
    const template = await pickWordTemplate(t("dialog.pickTemplate"));
    if (template !== null) setSettings((s) => ({ ...s, referenceDocx: template }));
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
              {t("app.clearList")}
            </button>
          )}
          <ContextMenuToggle />
          <LanguagePicker />
        </div>
      </header>

      {!insideTauri && (
        <p className="banner">{t("app.browserOnly", { command: "cargo tauri dev" })}</p>
      )}

      <DropZone hovering={hovering} compact={files.length > 0} disabled={busy} onPick={handlePickFiles} />

      {files.length > 0 && (
        <>
          <FileList files={files} busy={busy} onRemove={(p) => void converter.removeFile(p)} onReveal={(p) => void revealInFolder(p)} />

          <section className="panel">
            <h2 className="panel__title">{t("app.convertTo")}</h2>
            <TargetPicker
              targets={targets}
              locked={converter.locked}
              selected={target}
              disabled={busy}
              onSelect={setTarget}
            />
            {targets.length > 0 && (
              <SettingsPanel
                target={targets.find((t) => t.id === target) ?? null}
                fileCount={convertible}
                imagesOnly={imagesOnly}
                webPagesOnly={webPagesOnly}
                officeOnly={officeOnly}
                settings={settings}
                disabled={busy}
                onChange={setSettings}
                onPickFolder={handlePickFolder}
                onPickWordTemplate={handlePickWordTemplate}
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
              ? t("summary.done", { count: summary.converted }) +
                (summary.skipped > 0 ? t("summary.skipped", { count: summary.skipped }) : "") +
                (summary.failed > 0 ? t("summary.failed", { count: summary.failed }) : "")
              : tn("summary.convertible", files.length, { ready: convertible })}
          </span>
          <button type="button" className="button button--primary" onClick={handleConvert} disabled={!canConvert}>
            {busy ? t("app.converting") : t("app.convert")}
          </button>
        </footer>
      )}
    </main>
  );
}
