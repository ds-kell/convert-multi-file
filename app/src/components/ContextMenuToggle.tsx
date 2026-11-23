import { useEffect, useRef, useState } from "react";

import { contextMenuEnabled, insideTauri, setContextMenu } from "../api";
import { useI18n } from "../i18n";

const onWindows = navigator.userAgent.includes("Windows");

/**
 * Checkbox that adds/removes "Convert to" in the Explorer context menu.
 * Reads the real registry state on mount, so it reflects changes made by the
 * installer or `--register` too. When the UI language changes while the
 * menu is on, it is re-registered so its title follows the language.
 */
export function ContextMenuToggle() {
  const { language, t } = useI18n();
  const [enabled, setEnabled] = useState<boolean | null>(null); // null = loading
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!insideTauri || !onWindows) return;
    contextMenuEnabled()
      .then(setEnabled)
      .catch((e) => setError(String(e)));
  }, []);

  const apply = async (value: boolean) => {
    setSaving(true);
    setError(null);
    try {
      setEnabled(await setContextMenu(value, language));
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  // Re-title an enabled menu after a language switch (not on first load).
  const previousLanguage = useRef(language);
  useEffect(() => {
    if (previousLanguage.current === language) return;
    previousLanguage.current = language;
    if (enabled) void apply(true);
    // Runs on language changes only: `apply` and `enabled` are read as they are at that moment.
  }, [language]);

  // The menu is an Explorer (registry) integration; macOS and Linux have none yet.
  if (!onWindows) return null;

  return (
    <label className="checkbox" title={error ?? t("menu.tooltip")}>
      <input
        type="checkbox"
        checked={enabled ?? false}
        disabled={enabled === null || saving}
        onChange={(e) => void apply(e.target.checked)}
      />
      {t("menu.toggle")}
      {error !== null && <span className="status status--error">{t("menu.error")}</span>}
    </label>
  );
}
