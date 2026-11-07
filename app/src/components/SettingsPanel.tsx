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

const THEMES: { value: Theme; label: string }[] = [
  { value: "auto", label: "Tự động (theo hệ thống)" },
  { value: "light", label: "Sáng" },
  { value: "dark", label: "Tối" },
  { value: "none", label: "Không CSS" },
];

const OFFICE_ENGINES: { value: OfficeEngine; label: string }[] = [
  { value: "auto", label: "Tự động (Microsoft Office nếu có)" },
  { value: "microsoft", label: "Microsoft Office" },
  { value: "libreoffice", label: "LibreOffice" },
];

const PDF_PAGES: { value: PdfPage; label: string }[] = [
  { value: "a4", label: "A4 (tự xoay ngang với ảnh rộng)" },
  { value: "image", label: "Bằng kích thước ảnh" },
];

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
  const update = (patch: Partial<Settings>) => onChange({ ...settings, ...patch });

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
            Chất lượng: {settings.quality}
            {id === "pdf" && " (ảnh không trong suốt)"}
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
          Không nén mất dữ liệu (lossless)
        </label>
      )}

      {imageTarget && (
        <ResizeField value={settings.resize} disabled={disabled} onChange={(resize) => update({ resize })} />
      )}

      {imagePdf && (
        <>
          <label className="field">
            <span className="field__label">Khổ trang</span>
            <select
              value={settings.pdfPage}
              disabled={disabled}
              onChange={(e) => update({ pdfPage: e.target.value as PdfPage })}
            >
              {PDF_PAGES.map((page) => (
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
              Gộp {fileCount} ảnh thành 1 file PDF
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
          <span className="field__label">Bộ Office dùng để chuyển</span>
          <select
            value={settings.officeEngine}
            disabled={disabled}
            onChange={(e) => update({ officeEngine: e.target.value as OfficeEngine })}
          >
            {OFFICE_ENGINES.map((engine) => (
              <option key={engine.value} value={engine.value}>
                {engine.label}
              </option>
            ))}
          </select>
        </label>
      )}

      {id === "docx" && (
        <div className="field field--full">
          <span className="field__label">File Word mẫu (font, kiểu tiêu đề, lề trang)</span>
          <div className="field__row">
            <span className="path" title={settings.referenceDocx ?? undefined}>
              {settings.referenceDocx ?? "Mặc định của Pandoc"}
            </span>
            <button type="button" className="button button--secondary" onClick={onPickWordTemplate} disabled={disabled}>
              Chọn…
            </button>
            {settings.referenceDocx !== null && (
              <button
                type="button"
                className="link-button"
                onClick={() => update({ referenceDocx: null })}
                disabled={disabled}
              >
                Bỏ
              </button>
            )}
          </div>
        </div>
      )}

      {id === "html" && (
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

      <div className="field field--full">
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

const PAPERS: { value: Paper; label: string }[] = [
  { value: "a4", label: "A4" },
  { value: "a3", label: "A3" },
  { value: "a5", label: "A5" },
  { value: "letter", label: "Letter (Mỹ)" },
  { value: "legal", label: "Legal (Mỹ)" },
];

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
  const set = (patch: Partial<WebSetting>) => onChange({ ...value, ...patch });

  if (target === "png") {
    return (
      <label className="field">
        <span className="field__label">Chiều rộng cửa sổ chụp (px)</span>
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
        <span className="field__label">Khổ giấy</span>
        <select value={value.paper} disabled={disabled} onChange={(e) => set({ paper: e.target.value as Paper })}>
          {PAPERS.map((paper) => (
            <option key={paper.value} value={paper.value}>
              {paper.label}
            </option>
          ))}
        </select>
      </label>
      <label className="field">
        <span className="field__label">Lề (mm)</span>
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
        Khổ ngang
      </label>
      <label className="checkbox field--inline">
        <input
          type="checkbox"
          checked={value.printBackground}
          disabled={disabled}
          onChange={(e) => set({ printBackground: e.target.checked })}
        />
        In màu nền
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
      <span className="field__label">Kích thước</span>
      <div className="field__row">
        <select value={value.mode} disabled={disabled} onChange={(e) => setMode(e.target.value as ResizeSetting["mode"])}>
          <option value="original">Giữ nguyên</option>
          <option value="fit">Giới hạn tối đa (px)</option>
          <option value="percent">Theo phần trăm</option>
        </select>

        {value.mode === "fit" && (
          <>
            <input
              className="number"
              type="number"
              min={1}
              placeholder="rộng"
              aria-label="Chiều rộng tối đa"
              value={value.maxWidth ?? ""}
              disabled={disabled}
              onChange={(e) => onChange({ ...value, maxWidth: positiveInt(e.target.value) })}
            />
            <span aria-hidden="true">×</span>
            <input
              className="number"
              type="number"
              min={1}
              placeholder="cao"
              aria-label="Chiều cao tối đa"
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
              aria-label="Phần trăm"
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
