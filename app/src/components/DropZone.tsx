import { useI18n } from "../i18n";

interface DropZoneProps {
  /** Files are being dragged over the window. */
  hovering: boolean;
  /** Show the compact variant once files have been added. */
  compact: boolean;
  disabled: boolean;
  onPick: () => void;
}

/** Large drop target with a "choose files" button. */
export function DropZone({ hovering, compact, disabled, onPick }: DropZoneProps) {
  const { t } = useI18n();
  const className = ["dropzone", hovering && "dropzone--hover", compact && "dropzone--compact"]
    .filter(Boolean)
    .join(" ");

  return (
    <section className={className} aria-label={t("drop.label")}>
      <svg className="dropzone__icon" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M12 16V4m0 0-4 4m4-4 4 4M4 16v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2" />
      </svg>
      <p className="dropzone__text">{hovering ? t("drop.hover") : t("drop.prompt")}</p>
      <button type="button" className="button button--secondary" onClick={onPick} disabled={disabled}>
        {t("drop.pick")}
      </button>
    </section>
  );
}
