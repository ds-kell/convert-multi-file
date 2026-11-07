import { LANGUAGES, useI18n, type Language } from "../i18n";

/** Compact language selector shown in the header. */
export function LanguagePicker() {
  const { language, setLanguage, t } = useI18n();

  return (
    <select
      className="language-picker"
      aria-label={t("language.label")}
      title={t("language.label")}
      value={language}
      onChange={(e) => setLanguage(e.target.value as Language)}
    >
      {LANGUAGES.map((option) => (
        <option key={option.value} value={option.value}>
          {option.label}
        </option>
      ))}
    </select>
  );
}
