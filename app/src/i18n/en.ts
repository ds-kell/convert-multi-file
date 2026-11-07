// English UI texts. This file defines the set of message keys: every other
// language must provide exactly the same keys (enforced by the `Messages`
// type), so a missing translation is a compile error.
//
// Conventions:
// - `{name}` placeholders are replaced by `t(key, { name })`.
// - Keys ending in `.one` / `.other` are plural forms, used through
//   `tn(baseKey, count)`.

export const en = {
  // ----- Main window -----
  "app.clearList": "Clear list",
  "app.browserOnly": "Running in a web browser: start the app with “{command}” to convert files.",
  "app.convertTo": "Convert to",
  "app.convert": "Convert",
  "app.converting": "Converting…",

  "summary.done": "Done: {count} succeeded",
  "summary.skipped": ", {count} skipped",
  "summary.failed": ", {count} failed",
  "summary.convertible.one": "{ready} of {count} file can be converted",
  "summary.convertible.other": "{ready} of {count} files can be converted",

  "language.label": "Language",

  // ----- Drop zone -----
  "drop.label": "Add files",
  "drop.hover": "Drop the files here",
  "drop.prompt": "Drag files onto the window, or",
  "drop.pick": "Choose files…",

  // ----- File list -----
  "file.remove": "Remove {name}",
  "status.queued": "Waiting…",
  "status.progress": "Progress",
  "status.openFolder": "✓ Show in folder",
  "status.skipped": "Already {format}",

  "issue.unknownFormat": "Unrecognised file format",
  "issue.unsupported": "This format cannot be converted yet",
  "issue.needsTool": "Needs an extra program: {reason}",
  "issue.unreadable": "Cannot read the file: {detail}",

  // ----- Target picker -----
  "targets.label": "Output format",
  "targets.none": "The selected files have no output format in common. Choose files of the same kind (for example only images).",
  "targets.locked": "Unavailable: {reason}",

  // ----- Context menu toggle -----
  "menu.toggle": "Context menu",
  "menu.tooltip": "Add “Convert to” to the Explorer right-click menu",
  "menu.error": " (error)",

  // ----- Settings panel -----
  "settings.quality": "Quality: {value}",
  "settings.qualityPdfNote": " (opaque images)",
  "settings.lossless": "Lossless",
  "settings.size": "Size",
  "settings.resize.original": "Original size",
  "settings.resize.fit": "Fit within (px)",
  "settings.resize.percent": "Percentage",
  "settings.resize.widthPlaceholder": "width",
  "settings.resize.heightPlaceholder": "height",
  "settings.resize.maxWidth": "Maximum width",
  "settings.resize.maxHeight": "Maximum height",
  "settings.resize.percentLabel": "Percentage",

  "settings.pdfPage": "Page size",
  "settings.pdfPage.a4": "A4 (landscape for wide images)",
  "settings.pdfPage.image": "Same size as the image",
  "settings.merge.one": "Merge {count} file into one",
  "settings.merge.other": "Merge {count} files into one",
  "settings.mergeOnly": "These files can only be merged into one file.",
  "settings.stitch.direction": "Join",
  "settings.stitch.vertical": "Top to bottom",
  "settings.stitch.horizontal": "Left to right",
  "settings.stitch.gap": "Gap (px)",
  "settings.stitch.sameSize": "Same width/height",

  "settings.paper": "Paper size",
  "settings.paper.letter": "Letter (US)",
  "settings.paper.legal": "Legal (US)",
  "settings.margin": "Margin (mm)",
  "settings.landscape": "Landscape",
  "settings.background": "Print backgrounds",
  "settings.screenshotWidth": "Screenshot width (px)",

  "settings.officeEngine": "Office suite",
  "settings.office.auto": "Automatic (Microsoft Office if installed)",

  "settings.wordTemplate": "Word template (fonts, heading styles, margins)",
  "settings.wordTemplateDefault": "Pandoc default",
  "settings.choose": "Choose…",
  "settings.remove": "Remove",

  "settings.htmlTheme": "HTML theme",
  "settings.theme.auto": "Automatic (follows the system)",
  "settings.theme.light": "Light",
  "settings.theme.dark": "Dark",
  "settings.theme.none": "No stylesheet",

  "settings.saveTo": "Save to",
  "settings.sameFolder": "Same folder as the original",
  "settings.change": "Change…",
  "settings.default": "Default",

  // ----- Quick-convert window -----
  "quick.windowTitle": "ConvertApp – Quick convert",
  "quick.receiving": "Receiving files…",
  "quick.converting.one": "Converting {count} file to {target}…",
  "quick.converting.other": "Converting {count} files to {target}…",
  "quick.finished": "Finished",
  "quick.succeeded": "{count} succeeded",
  "quick.autoClose": "Close when done",
  "quick.openFolder": "Show in folder",
  "quick.close": "Close",

  // ----- File dialogs -----
  "dialog.pickFiles": "Choose files to convert",
  "dialog.pickTemplate": "Choose a Word template (fonts, heading styles, margins…)",
  "dialog.pickFolder": "Choose where to save the results",
} as const;

/** Every message key. */
export type MessageKey = keyof typeof en;

/** A complete set of translations. */
export type Messages = Record<MessageKey, string>;
