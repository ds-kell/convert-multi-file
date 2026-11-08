import { useState } from "react";

import { copyText, openInBrowser } from "../api";
import { useI18n } from "../i18n";
import type { LockedTarget, Requirement } from "../types";

interface MissingToolsProps {
  /** Targets that need a program that is not installed. */
  locked: LockedTarget[];
  busy: boolean;
  /** Checks the system again after the user installed something. */
  onRecheck: () => void;
}

/**
 * Explains which formats need an extra program, with a download link and
 * the install command for each, and a button to check again once installed.
 * Targets needing the same programs are grouped on one line.
 */
export function MissingTools({ locked, busy, onRecheck }: MissingToolsProps) {
  const { t } = useI18n();
  if (locked.length === 0) return null;

  // Group formats by what they need, keeping first-seen order.
  const groups = new Map<string, { formats: string[]; alternatives: Requirement[][] }>();
  for (const { format, alternatives } of locked) {
    const key = JSON.stringify(alternatives);
    const group = groups.get(key) ?? { formats: [], alternatives };
    group.formats.push(format.id.toUpperCase());
    groups.set(key, group);
  }

  return (
    <div className="missing-tools" role="note">
      <p className="missing-tools__title">{t("tools.title")}</p>
      {[...groups.values()].map((group) => (
        <div key={group.formats.join()} className="missing-tools__group">
          <span className="missing-tools__formats">🔒 {group.formats.join(", ")}</span>
          <div className="missing-tools__options">
            {group.alternatives.map((option, index) => (
              <div key={option.map((r) => r.name).join("+")} className="missing-tools__option">
                {index > 0 && <span className="missing-tools__or">{t("tools.or")}</span>}
                {option.map((requirement, i) => (
                  <span key={requirement.name} className="missing-tools__requirement">
                    {i > 0 && <span aria-hidden="true"> + </span>}
                    <RequirementItem requirement={requirement} />
                  </span>
                ))}
              </div>
            ))}
          </div>
        </div>
      ))}
      <button type="button" className="button button--secondary" onClick={onRecheck} disabled={busy}>
        {t("tools.recheck")}
      </button>
    </div>
  );
}

/** One program: name, download link and (when available) its install command. */
function RequirementItem({ requirement }: { requirement: Requirement }) {
  const { t } = useI18n();
  const [copied, setCopied] = useState(false);

  const copy = async (command: string) => {
    await copyText(command);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1500);
  };

  return (
    <>
      <strong>{requirement.name}</strong>
      {!requirement.free && <span className="status status--muted"> ({t("tools.paid")})</span>}{" "}
      <button type="button" className="link-button" onClick={() => void openInBrowser(requirement.url)}>
        {t("tools.download")}
      </button>
      {requirement.installCommand !== null && (
        <>
          {" · "}
          <button
            type="button"
            className="link-button"
            title={requirement.installCommand}
            onClick={() => void copy(requirement.installCommand!)}
          >
            {copied ? t("tools.copied") : t("tools.copy")}
          </button>
        </>
      )}
    </>
  );
}

/** Short text for a file list cell: "LibreOffice or Microsoft Office". */
export function describeAlternatives(alternatives: Requirement[][], or: string): string {
  return alternatives.map((option) => option.map((r) => r.name).join(" + ")).join(` ${or} `);
}
