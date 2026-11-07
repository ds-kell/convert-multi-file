import { useEffect, useState } from "react";

import { closeWindow, revealInFolder, setWindowTitle } from "./api";
import { Status } from "./components/FileList";
import { useQuickConvert } from "./hooks/useQuickConvert";
import { useI18n } from "./i18n";

/** localStorage key remembering the "close when done" checkbox. */
const AUTO_CLOSE_KEY = "quick.autoClose";
/** Delay before auto-closing, so the user sees the result. */
const AUTO_CLOSE_DELAY_MS = 1500;

function loadAutoClose(): boolean {
  try {
    return localStorage.getItem(AUTO_CLOSE_KEY) !== "false";
  } catch {
    return true; // storage unavailable: use the default
  }
}

function saveAutoClose(value: boolean) {
  try {
    localStorage.setItem(AUTO_CLOSE_KEY, String(value));
  } catch {
    // Not critical: the preference simply is not remembered.
  }
}

/**
 * Small window opened by the Explorer "Convert to" menu: shows progress for
 * the files being converted and closes itself when everything succeeded.
 */
export default function QuickApp() {
  const { t, tn } = useI18n();
  const { rows, running, error, done, failed, skipped } = useQuickConvert();

  // The backend creates the window with a neutral title; localise it here.
  useEffect(() => {
    void setWindowTitle(t("quick.windowTitle"));
  }, [t]);
  const [autoClose, setAutoClose] = useState(loadAutoClose);

  const finished = !running && rows.length > 0;
  // Every row must have reached a successful end state: a row still "queued"
  // after the run means progress was lost, which must stay visible.
  const allSucceeded =
    finished && error === null && rows.every((r) => r.status.kind === "done" || r.status.kind === "skipped");
  const firstOutput = rows.find((r) => r.status.kind === "done")?.status;

  // Close automatically after a fully successful run (cancelled if new
  // files arrive, because `running` turns true again).
  useEffect(() => {
    if (!autoClose || !allSucceeded) return;
    const timer = window.setTimeout(() => void closeWindow(), AUTO_CLOSE_DELAY_MS);
    return () => window.clearTimeout(timer);
  }, [autoClose, allSucceeded]);

  const toggleAutoClose = (value: boolean) => {
    setAutoClose(value);
    saveAutoClose(value);
  };

  const target = rows[rows.length - 1]?.target;
  const title =
    rows.length === 0
      ? t("quick.receiving")
      : running && target
        ? tn("quick.converting", rows.length, { target: target.id.toUpperCase() })
        : t("quick.finished");

  return (
    <main className="quick">
      <header className="quick__header">
        <h1>{title}</h1>
        {finished && (
          <p className="summary">
            {t("quick.succeeded", { count: done })}
            {skipped > 0 && t("summary.skipped", { count: skipped })}
            {failed > 0 && t("summary.failed", { count: failed })}
          </p>
        )}
      </header>

      <ul className="file-list quick__list">
        {rows.map((row) => (
          <li key={row.key} className="file-row file-row--quick" title={row.path}>
            <span className="file-row__badge">{row.target.id}</span>
            <span className="file-row__name">{row.name}</span>
            <span className="file-row__status">
              <Status status={row.status} onReveal={(p) => void revealInFolder(p)} />
            </span>
          </li>
        ))}
      </ul>

      {error !== null && <p className="banner banner--error">{error}</p>}

      <footer className="quick__footer">
        <label className="checkbox">
          <input type="checkbox" checked={autoClose} onChange={(e) => toggleAutoClose(e.target.checked)} />
          {t("quick.autoClose")}
        </label>
        <div className="quick__actions">
          {firstOutput?.kind === "done" && (
            <button type="button" className="button button--secondary" onClick={() => void revealInFolder(firstOutput.output)}>
              {t("quick.openFolder")}
            </button>
          )}
          <button type="button" className="button button--primary" onClick={() => void closeWindow()} disabled={running}>
            {t("quick.close")}
          </button>
        </div>
      </footer>
    </main>
  );
}
