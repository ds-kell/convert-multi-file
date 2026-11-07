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
  if (targets.length === 0 && locked.length === 0) {
    return (
      <p className="hint">
        Không có định dạng chung cho các file đã chọn. Hãy chọn các file cùng loại (ví dụ toàn ảnh).
      </p>
    );
  }

  // One line per distinct reason, e.g. "Pandoc is not installed…".
  const reasons = [...new Set(locked.map((l) => l.reason))];

  return (
    <>
      <div className="targets" role="radiogroup" aria-label="Định dạng đầu ra">
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
          {reason}
        </p>
      ))}
    </>
  );
}
