import { useI18n } from "../i18n";
import type { FormatInfo, LockedTarget } from "../types";

interface TargetPickerProps {
  targets: FormatInfo[];
  /** Targets that need a missing program: shown disabled with the reason. */
  locked: LockedTarget[];
  selected: string | null;
  disabled: boolean;
  onSelect: (id: string) => void;
}

/** Segmented buttons for choosing the output format. */
export function TargetPicker({ targets, locked, selected, disabled, onSelect }: TargetPickerProps) {
  const { t } = useI18n();

  if (targets.length === 0 && locked.length === 0) {
    return <p className="hint">{t("targets.none")}</p>;
  }

  // One line per distinct reason, e.g. "Pandoc is not installed…". Reasons
  // come from the engine in English, with the exact install command.
  const reasons = [...new Set(locked.map((l) => l.reason))];

  return (
    <>
      <div className="targets" role="radiogroup" aria-label={t("targets.label")}>
        {targets.map((target) => (
          <button
            key={target.id}
            type="button"
            role="radio"
            aria-checked={selected === target.id}
            className={selected === target.id ? "chip chip--selected" : "chip"}
            onClick={() => onSelect(target.id)}
            disabled={disabled}
            title={target.name}
          >
            {target.id.toUpperCase()}
          </button>
        ))}
        {locked.map(({ format, reason }) => (
          <button key={format.id} type="button" className="chip chip--locked" disabled title={reason}>
            🔒 {format.id.toUpperCase()}
          </button>
        ))}
      </div>
      {reasons.map((reason) => (
        <p key={reason} className="hint">
          {t("targets.locked", { reason })}
        </p>
      ))}
    </>
  );
}
