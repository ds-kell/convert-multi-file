import { useI18n } from "../i18n";
import type { FileIssue, FileItem, FileStatus } from "../types";
import { describeAlternatives } from "./MissingTools";

interface FileListProps {
  files: FileItem[];
  busy: boolean;
  onRemove: (path: string) => void;
  onReveal: (path: string) => void;
}

/** Table of the selected files with their format and conversion status. */
export function FileList({ files, busy, onRemove, onReveal }: FileListProps) {
  const { t } = useI18n();

  return (
    <ul className="file-list">
      {files.map((file) => (
        <li key={file.path} className="file-row" title={file.path}>
          <span className="file-row__badge">{file.format?.id ?? "?"}</span>
          <span className="file-row__name">{file.name}</span>
          <span className="file-row__status">
            {file.error !== null ? (
              <Issue issue={file.error} />
            ) : (
              <Status status={file.status} onReveal={onReveal} />
            )}
          </span>
          <button
            type="button"
            className="icon-button"
            aria-label={t("file.remove", { name: file.name })}
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

/** Why a file cannot be converted, in the UI language. */
function Issue({ issue }: { issue: FileIssue }) {
  const { t } = useI18n();
  const text = (() => {
    switch (issue.kind) {
      case "unknownFormat":
        return t("issue.unknownFormat");
      case "unsupported":
        return t("issue.unsupported");
      case "needsTool":
        return t("issue.needsTool", { tools: describeAlternatives(issue.alternatives, t("tools.or")) });
      case "unreadable":
        return t("issue.unreadable", { detail: issue.detail });
    }
  })();
  return (
    <span className="status status--error" title={text}>
      {text}
    </span>
  );
}

/** Status cell: progress bar, result link or error message. */
export function Status({ status, onReveal }: { status: FileStatus; onReveal: (path: string) => void }) {
  const { t } = useI18n();

  switch (status.kind) {
    case "idle":
      return null;
    case "queued":
      return <span className="status status--muted">{t("status.queued")}</span>;
    case "converting":
      return <progress className="progress" max={1} value={status.fraction} aria-label={t("status.progress")} />;
    case "done":
      return (
        <button type="button" className="link-button" onClick={() => onReveal(status.output)}>
          {t("status.openFolder")}
        </button>
      );
    case "skipped":
      return <span className="status status--muted">{t("status.skipped", { format: status.format })}</span>;
    case "failed":
      // Engine/tool error text: technical, kept as reported (English).
      return (
        <span className="status status--error" title={status.message}>
          {status.message}
        </span>
      );
  }
}
