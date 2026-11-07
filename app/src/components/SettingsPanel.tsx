import { useI18n } from "../i18n";
import type {
  FormatInfo,
  OfficeEngine,
  Paper,
  PdfPage,
  ResizeSetting,
  Settings,
  Theme,
  WebSetting,
} from "../types";

interface SettingsPanelProps {
  /** Selected target format; decides which options are shown. */
  target: FormatInfo | null;
  /** Number of files that will be converted (for the PDF merge option). */
  fileCount: number;
  /** All files to convert are images (image options only apply then). */
  imagesOnly: boolean;
  /** All files are web pages (HTML/Markdown): page setup options apply. */
  webPagesOnly: boolean;
  /** All files are office documents: the office suite choice applies. */
  officeOnly: boolean;
  settings: Settings;
  disabled: boolean;
  onChange: (settings: Settings) => void;
  onPickFolder: () => void;
  onPickWordTemplate: () => void;
}

/** Options relevant to the chosen target, plus the output folder. */
export function SettingsPanel({
  target,
  fileCount,
  imagesOnly,
  webPagesOnly,
  officeOnly,
  settings,
  disabled,
  onChange,
  onPickFolder,
  onPickWordTemplate,
}: SettingsPanelProps) {
  const { t, tn } = useI18n();
  const update = (patch: Partial<Settings>) => onChange({ ...settings, ...patch });

  const themes: { value: Theme; label: string }[] = [
    { value: "auto", label: t("settings.theme.auto") },
    { value: "light", label: t("settings.theme.light") },
    { value: "dark", label: t("settings.theme.dark") },
    { value: "none", label: t("settings.theme.none") },
  ];
  // Product names are not translated.
  const officeEngines: { value: OfficeEngine; label: string }[] = [
    { value: "auto", label: t("settings.office.auto") },
    { value: "microsoft", label: "Microsoft Office" },
    { value: "libreoffice", label: "LibreOffice" },
  ];
  const pdfPages: { value: PdfPage; label: string }[] = [
    { value: "a4", label: t("settings.pdfPage.a4") },
    { value: "image", label: t("settings.pdfPage.image") },
  ];

  const id = target?.id;
  // Image options apply to image outputs, and to PDFs made from images.
  // Web pages (HTML/Markdown) rendered by the browser have page options instead.
  const imageTarget = imagesOnly && (target?.category === "image" || id === "pdf");
  const webRender = webPagesOnly && (id === "pdf" || id === "png");
  const imagePdf = imagesOnly && id === "pdf";
  const showQuality = imagesOnly && (id === "jpg" || (id === "webp" && !settings.webpLossless) || id === "pdf");

  return (
    <div className="settings">
      {showQuality && (
        <label className="field">
          <span className="field__label">
            {t("settings.quality", { value: settings.quality })}
            {id === "pdf" && t("settings.qualityPdfNote")}
          </span>
          <input
            type="range"
            min={1}
            max={100}
            value={settings.quality}
            disabled={disabled}
            onChange={(e) => update({ quality: Number(e.target.value) })}
          />
        </label>
      )}

      {id === "webp" && (
        <label className="checkbox field--inline">
          <input
            type="checkbox"
            checked={settings.webpLossless}
            disabled={disabled}
            onChange={(e) => update({ webpLossless: e.target.checked })}
          />
          {t("settings.lossless")}
        </label>
      )}

      {imageTarget && (
        <ResizeField value={settings.resize} disabled={disabled} onChange={(resize) => update({ resize })} />
      )}

      {imagePdf && (
        <>
          <label className="field">
            <span className="field__label">{t("settings.pdfPage")}</span>
            <select
              value={settings.pdfPage}
              disabled={disabled}
              onChange={(e) => update({ pdfPage: e.target.value as PdfPage })}
            >
              {pdfPages.map((page) => (
                <option key={page.value} value={page.value}>
                  {page.label}
                </option>
              ))}
            </select>
          </label>
          {fileCount > 1 && (
            <label className="checkbox field--inline">
              <input
                type="checkbox"
                checked={settings.mergePdf}
                disabled={disabled}
                onChange={(e) => update({ mergePdf: e.target.checked })}
              />
              {tn("settings.merge", fileCount)}
            </label>
          )}
        </>
      )}

      {webRender && (
        <WebFields
          target={id}
          value={settings.web}
          disabled={disabled}
          onChange={(web) => update({ web })}
        />
      )}

      {officeOnly && target !== null && (
        <label className="field">
          <span className="field__label">{t("settings.officeEngine")}</span>
          <select
            value={settings.officeEngine}
            disabled={disabled}
            onChange={(e) => update({ officeEngine: e.target.value as OfficeEngine })}
          >
            {officeEngines.map((engine) => (
              <option key={engine.value} value={engine.value}>
                {engine.label}
              </option>
            ))}
          </select>
        </label>
      )}

      {id === "docx" && (
        <div className="field field--full">
          <span className="field__label">{t("settings.wordTemplate")}</span>
          <div className="field__row">
            <span className="path" title={settings.referenceDocx ?? undefined}>
              {settings.referenceDocx ?? t("settings.wordTemplateDefault")}
            </span>
            <button type="button" className="button button--secondary" onClick={onPickWordTemplate} disabled={disabled}>
              {t("settings.choose")}
            </button>
            {settings.referenceDocx !== null && (
              <button
                type="button"
                className="link-button"
                onClick={() => update({ referenceDocx: null })}
                disabled={disabled}
              >
                {t("settings.remove")}
              </button>
            )}
          </div>
        </div>
      )}

      {id === "html" && (
        <label className="field">
          <span className="field__label">{t("settings.htmlTheme")}</span>
          <select
            value={settings.theme}
            disabled={disabled}
            onChange={(e) => update({ theme: e.target.value as Theme })}
          >
            {themes.map((theme) => (
              <option key={theme.value} value={theme.value}>
                {theme.label}
              </option>
            ))}
          </select>
        </label>
      )}

      <div className="field field--full">
        <span className="field__label">{t("settings.saveTo")}</span>
        <div className="field__row">
          <span className="path" title={settings.outputDir ?? undefined}>
            {settings.outputDir ?? t("settings.sameFolder")}
          </span>
          <button type="button" className="button button--secondary" onClick={onPickFolder} disabled={disabled}>
            {t("settings.change")}
          </button>
          {settings.outputDir !== null && (
            <button
              type="button"
              className="link-button"
              onClick={() => update({ outputDir: null })}
              disabled={disabled}
            >
              {t("settings.default")}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

/** Clamps a number input to `[min, max]`, falling back to `fallback`. */
function clampInt(value: string, min: number, max: number, fallback: number): number {
  const n = Number.parseInt(value, 10);
  return Number.isFinite(n) ? Math.min(max, Math.max(min, n)) : fallback;
}

/** Page setup for HTML/Markdown printed to PDF or captured as PNG. */
function WebFields({
  target,
  value,
  disabled,
  onChange,
}: {
  target: "pdf" | "png";
  value: WebSetting;
  disabled: boolean;
  onChange: (value: WebSetting) => void;
}) {
  const { t } = useI18n();
  const set = (patch: Partial<WebSetting>) => onChange({ ...value, ...patch });
  const papers: { value: Paper; label: string }[] = [
    { value: "a4", label: "A4" },
    { value: "a3", label: "A3" },
    { value: "a5", label: "A5" },
    { value: "letter", label: t("settings.paper.letter") },
    { value: "legal", label: t("settings.paper.legal") },
  ];

  if (target === "png") {
    return (
      <label className="field">
        <span className="field__label">{t("settings.screenshotWidth")}</span>
        <input
          className="number"
          type="number"
          min={320}
          max={4096}
          value={value.viewportWidth}
          disabled={disabled}
          onChange={(e) => set({ viewportWidth: clampInt(e.target.value, 320, 4096, 1280) })}
        />
      </label>
    );
  }

  return (
    <>
      <label className="field">
        <span className="field__label">{t("settings.paper")}</span>
        <select value={value.paper} disabled={disabled} onChange={(e) => set({ paper: e.target.value as Paper })}>
          {papers.map((paper) => (
            <option key={paper.value} value={paper.value}>
              {paper.label}
            </option>
          ))}
        </select>
      </label>
      <label className="field">
        <span className="field__label">{t("settings.margin")}</span>
        <input
          className="number"
          type="number"
          min={0}
          max={100}
          value={value.marginMm}
          disabled={disabled}
          onChange={(e) => set({ marginMm: clampInt(e.target.value, 0, 100, 10) })}
        />
      </label>
      <label className="checkbox field--inline">
        <input
          type="checkbox"
          checked={value.landscape}
          disabled={disabled}
          onChange={(e) => set({ landscape: e.target.checked })}
        />
        {t("settings.landscape")}
      </label>
      <label className="checkbox field--inline">
        <input
          type="checkbox"
          checked={value.printBackground}
          disabled={disabled}
          onChange={(e) => set({ printBackground: e.target.checked })}
        />
        {t("settings.background")}
      </label>
    </>
  );
}

/** Parses a positive integer from an input, or `null` when empty/invalid. */
function positiveInt(value: string): number | null {
  const n = Number.parseInt(value, 10);
  return Number.isFinite(n) && n > 0 ? n : null;
}

/** Resize mode selector with the inputs for the chosen mode. */
function ResizeField({
  value,
  disabled,
  onChange,
}: {
  value: ResizeSetting;
  disabled: boolean;
  onChange: (value: ResizeSetting) => void;
}) {
  const { t } = useI18n();
  const setMode = (mode: ResizeSetting["mode"]) => {
    switch (mode) {
      case "original":
        return onChange({ mode });
      case "fit":
        return onChange({ mode, maxWidth: 1920, maxHeight: 1920 });
      case "percent":
        return onChange({ mode, percent: 50 });
    }
  };

  return (
    <div className="field">
      <span className="field__label">{t("settings.size")}</span>
      <div className="field__row">
        <select value={value.mode} disabled={disabled} onChange={(e) => setMode(e.target.value as ResizeSetting["mode"])}>
          <option value="original">{t("settings.resize.original")}</option>
          <option value="fit">{t("settings.resize.fit")}</option>
          <option value="percent">{t("settings.resize.percent")}</option>
        </select>

        {value.mode === "fit" && (
          <>
            <input
              className="number"
              type="number"
              min={1}
              placeholder={t("settings.resize.widthPlaceholder")}
              aria-label={t("settings.resize.maxWidth")}
              value={value.maxWidth ?? ""}
              disabled={disabled}
              onChange={(e) => onChange({ ...value, maxWidth: positiveInt(e.target.value) })}
            />
            <span aria-hidden="true">×</span>
            <input
              className="number"
              type="number"
              min={1}
              placeholder={t("settings.resize.heightPlaceholder")}
              aria-label={t("settings.resize.maxHeight")}
              value={value.maxHeight ?? ""}
              disabled={disabled}
              onChange={(e) => onChange({ ...value, maxHeight: positiveInt(e.target.value) })}
            />
          </>
        )}

        {value.mode === "percent" && (
          <>
            <input
              className="number"
              type="number"
              min={1}
              max={1000}
              aria-label={t("settings.resize.percentLabel")}
              value={value.percent}
              disabled={disabled}
              onChange={(e) => onChange({ mode: "percent", percent: positiveInt(e.target.value) ?? 100 })}
            />
            <span>%</span>
          </>
        )}
      </div>
    </div>
  );
}
