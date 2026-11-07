import type { Settings, Theme } from "../types";

interface SettingsPanelProps {
  /** Selected target format id, decides which options are shown. */
  target: string | null;
  settings: Settings;
  disabled: boolean;
  onChange: (settings: Settings) => void;
  onPickFolder: () => void;
}

const THEMES: { value: Theme; label: string }[] = [
  { value: "auto", label: "Tự động (theo hệ thống)" },
  { value: "light", label: "Sáng" },
  { value: "dark", label: "Tối" },
  { value: "none", label: "Không CSS" },
];

/** Options relevant to the chosen target, plus the output folder. */
export function SettingsPanel({ target, settings, disabled, onChange, onPickFolder }: SettingsPanelProps) {
  const update = (patch: Partial<Settings>) => onChange({ ...settings, ...patch });

  return (
    <div className="settings">
      {target === "jpg" && (
        <label className="field">
          <span className="field__label">Chất lượng JPEG: {settings.jpegQuality}</span>
          <input
            type="range"
            min={1}
            max={100}
            value={settings.jpegQuality}
            disabled={disabled}
            onChange={(e) => update({ jpegQuality: Number(e.target.value) })}
          />
        </label>
      )}

      {target === "html" && (
        <label className="field">
          <span className="field__label">Giao diện HTML</span>
          <select
            value={settings.theme}
            disabled={disabled}
            onChange={(e) => update({ theme: e.target.value as Theme })}
          >
            {THEMES.map((theme) => (
              <option key={theme.value} value={theme.value}>
                {theme.label}
              </option>
            ))}
          </select>
        </label>
      )}

      <div className="field">
        <span className="field__label">Lưu vào</span>
        <div className="field__row">
          <span className="path" title={settings.outputDir ?? undefined}>
            {settings.outputDir ?? "Cùng thư mục với file gốc"}
          </span>
          <button type="button" className="button button--secondary" onClick={onPickFolder} disabled={disabled}>
            Đổi…
          </button>
          {settings.outputDir !== null && (
            <button
              type="button"
              className="link-button"
              onClick={() => update({ outputDir: null })}
              disabled={disabled}
            >
              Mặc định
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
