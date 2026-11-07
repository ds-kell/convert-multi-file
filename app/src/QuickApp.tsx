import { useEffect, useState } from "react";

import { closeWindow, revealInFolder } from "./api";
import { Status } from "./components/FileList";
import { useQuickConvert } from "./hooks/useQuickConvert";

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
  const { rows, running, error, done, failed, skipped } = useQuickConvert();
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
  const title = running
    ? `Đang chuyển ${rows.length} file${target ? ` sang ${target.id.toUpperCase()}` : ""}…`
    : rows.length === 0
      ? "Đang nhận file…"
      : "Hoàn tất";

  return (
    <main className="quick">
      <header className="quick__header">
        <h1>{title}</h1>
        {finished && (
          <p className="summary">
            {done} thành công
            {skipped > 0 && `, ${skipped} bỏ qua`}
            {failed > 0 && `, ${failed} lỗi`}
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
          Tự đóng khi xong
        </label>
        <div className="quick__actions">
          {firstOutput?.kind === "done" && (
            <button type="button" className="button button--secondary" onClick={() => void revealInFolder(firstOutput.output)}>
              Mở thư mục
            </button>
          )}
          <button type="button" className="button button--primary" onClick={() => void closeWindow()} disabled={running}>
            Đóng
          </button>
        </div>
      </footer>
    </main>
  );
}
