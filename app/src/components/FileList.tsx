import type { FileItem, FileStatus } from "../types";

interface FileListProps {
  files: FileItem[];
  busy: boolean;
  onRemove: (path: string) => void;
  onReveal: (path: string) => void;
}

/** Table of the selected files with their format and conversion status. */
export function FileList({ files, busy, onRemove, onReveal }: FileListProps) {
  return (
    <ul className="file-list">
      {files.map((file) => (
        <li key={file.path} className="file-row" title={file.path}>
          <span className="file-row__badge">{file.format?.id ?? "?"}</span>
          <span className="file-row__name">{file.name}</span>
          <span className="file-row__status">
            {file.error !== null ? (
              <span className="status status--error">{file.error}</span>
            ) : (
              <Status status={file.status} onReveal={onReveal} />
            )}
          </span>
          <button
            type="button"
            className="icon-button"
            aria-label={`Bỏ ${file.name}`}
            onClick={() => onRemove(file.path)}
            disabled={busy}
          >
            ×
          </button>
        </li>
      ))}
    </ul>
  );
}

/** Status cell: progress bar, result link or error message. */
function Status({ status, onReveal }: { status: FileStatus; onReveal: (path: string) => void }) {
  switch (status.kind) {
    case "idle":
      return null;
    case "queued":
      return <span className="status status--muted">Đang chờ…</span>;
    case "converting":
      return (
        <progress className="progress" max={1} value={status.fraction} aria-label="Tiến độ" />
      );
    case "done":
      return (
        <button type="button" className="link-button" onClick={() => onReveal(status.output)}>
          ✓ Mở thư mục
        </button>
      );
    case "skipped":
      return <span className="status status--muted">{status.reason}</span>;
    case "failed":
      return (
        <span className="status status--error" title={status.message}>
          {status.message}
        </span>
      );
  }
}
