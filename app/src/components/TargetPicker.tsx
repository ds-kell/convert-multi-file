import type { FormatInfo } from "../types";

interface TargetPickerProps {
  targets: FormatInfo[];
  selected: string | null;
  disabled: boolean;
  onSelect: (id: string) => void;
}

/** Segmented buttons for choosing the output format. */
export function TargetPicker({ targets, selected, disabled, onSelect }: TargetPickerProps) {
  if (targets.length === 0) {
    return (
      <p className="hint">
        Không có định dạng chung cho các file đã chọn. Hãy chọn các file cùng loại (ví dụ toàn ảnh).
      </p>
    );
  }

  return (
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
    </div>
  );
}
