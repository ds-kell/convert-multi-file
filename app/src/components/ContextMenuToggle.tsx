import { useEffect, useState } from "react";

import { contextMenuEnabled, insideTauri, setContextMenu } from "../api";

/**
 * Checkbox that adds/removes "Convert to" in the Explorer context menu.
 * Reads the real registry state on mount, so it reflects changes made by the
 * installer or `--register` too.
 */
export function ContextMenuToggle() {
  const [enabled, setEnabled] = useState<boolean | null>(null); // null = loading
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!insideTauri) return;
    contextMenuEnabled()
      .then(setEnabled)
      .catch((e) => setError(String(e)));
  }, []);

  const toggle = async (value: boolean) => {
    setSaving(true);
    setError(null);
    try {
      setEnabled(await setContextMenu(value));
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <label className="checkbox" title={error ?? "Thêm mục “Convert to” khi chuột phải vào file trong Explorer"}>
      <input
        type="checkbox"
        checked={enabled ?? false}
        disabled={enabled === null || saving}
        onChange={(e) => void toggle(e.target.checked)}
      />
      Menu chuột phải
      {error !== null && <span className="status status--error"> (lỗi)</span>}
    </label>
  );
}
