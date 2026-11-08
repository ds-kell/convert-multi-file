// Vietnamese UI texts. Must define exactly the keys of `en.ts`.
// Vietnamese has no grammatical plural, so `.one` and `.other` are the same.

import type { Messages } from "./en";

export const vi: Messages = {
  // ----- Main window -----
  "app.clearList": "Xoá danh sách",
  "app.browserOnly": "Đang chạy trong trình duyệt: hãy mở app bằng “{command}” để chuyển đổi file.",
  "app.convertTo": "Chuyển sang",
  "app.convert": "Chuyển đổi",
  "app.converting": "Đang chuyển…",

  "summary.done": "Xong: {count} thành công",
  "summary.skipped": ", {count} bỏ qua",
  "summary.failed": ", {count} lỗi",
  "summary.convertible.one": "{ready}/{count} file có thể chuyển",
  "summary.convertible.other": "{ready}/{count} file có thể chuyển",

  "language.label": "Ngôn ngữ",

  // ----- Drop zone -----
  "drop.label": "Thêm file",
  "drop.hover": "Thả file vào đây",
  "drop.prompt": "Kéo-thả file vào cửa sổ, hoặc",
  "drop.pick": "Chọn file…",

  // ----- File list -----
  "file.remove": "Bỏ {name}",
  "status.queued": "Đang chờ…",
  "status.progress": "Tiến độ",
  "status.openFolder": "✓ Mở thư mục",
  "status.skipped": "Đã là {format}",

  "issue.unknownFormat": "Không nhận diện được định dạng",
  "issue.unsupported": "Chưa hỗ trợ chuyển đổi định dạng này",
  "issue.needsTool": "Cần cài thêm {tools}",
  "issue.unreadable": "Không đọc được file: {detail}",

  // ----- Target picker -----
  "targets.label": "Định dạng đầu ra",
  "targets.none": "Các file đã chọn không có định dạng đích chung. Hãy chọn các file cùng loại (ví dụ toàn ảnh).",

  // ----- Missing external programs -----
  "tools.title": "Một số định dạng cần cài thêm chương trình:",
  "tools.or": "hoặc",
  "tools.download": "Tải về",
  "tools.copy": "Sao chép lệnh cài",
  "tools.copied": "Đã sao chép!",
  "tools.paid": "trả phí",
  "tools.recheck": "Cài xong? Kiểm tra lại",

  // ----- Context menu toggle -----
  "menu.toggle": "Menu chuột phải",
  "menu.tooltip": "Thêm mục “Chuyển sang” khi chuột phải vào file trong Explorer",
  "menu.error": " (lỗi)",

  // ----- Settings panel -----
  "settings.quality": "Chất lượng: {value}",
  "settings.qualityPdfNote": " (ảnh không trong suốt)",
  "settings.lossless": "Không nén mất dữ liệu (lossless)",
  "settings.size": "Kích thước",
  "settings.resize.original": "Giữ nguyên",
  "settings.resize.fit": "Giới hạn tối đa (px)",
  "settings.resize.percent": "Theo phần trăm",
  "settings.resize.widthPlaceholder": "rộng",
  "settings.resize.heightPlaceholder": "cao",
  "settings.resize.maxWidth": "Chiều rộng tối đa",
  "settings.resize.maxHeight": "Chiều cao tối đa",
  "settings.resize.percentLabel": "Phần trăm",

  "settings.pdfPage": "Khổ trang",
  "settings.pdfPage.a4": "A4 (tự xoay ngang với ảnh rộng)",
  "settings.pdfPage.image": "Bằng kích thước ảnh",
  "settings.merge.one": "Gộp {count} file thành 1",
  "settings.merge.other": "Gộp {count} file thành 1",
  "settings.mergeOnly": "Các file này chỉ có thể gộp thành 1 file.",
  "settings.stitch.direction": "Nối",
  "settings.stitch.vertical": "Từ trên xuống",
  "settings.stitch.horizontal": "Từ trái sang phải",
  "settings.stitch.gap": "Khoảng cách (px)",
  "settings.stitch.sameSize": "Cùng chiều rộng/cao",

  "settings.paper": "Khổ giấy",
  "settings.paper.letter": "Letter (Mỹ)",
  "settings.paper.legal": "Legal (Mỹ)",
  "settings.margin": "Lề (mm)",
  "settings.landscape": "Khổ ngang",
  "settings.background": "In màu nền",
  "settings.screenshotWidth": "Chiều rộng cửa sổ chụp (px)",

  "settings.officeEngine": "Bộ Office dùng để chuyển",
  "settings.office.auto": "Tự động (Microsoft Office nếu có)",

  "settings.wordTemplate": "File Word mẫu (font, kiểu tiêu đề, lề trang)",
  "settings.wordTemplateDefault": "Mặc định của Pandoc",
  "settings.choose": "Chọn…",
  "settings.remove": "Bỏ",

  "settings.htmlTheme": "Giao diện HTML",
  "settings.theme.auto": "Tự động (theo hệ thống)",
  "settings.theme.light": "Sáng",
  "settings.theme.dark": "Tối",
  "settings.theme.none": "Không CSS",

  "settings.saveTo": "Lưu vào",
  "settings.sameFolder": "Cùng thư mục với file gốc",
  "settings.change": "Đổi…",
  "settings.default": "Mặc định",

  // ----- Quick-convert window -----
  "quick.windowTitle": "ConvertApp – Chuyển đổi nhanh",
  "quick.receiving": "Đang nhận file…",
  "quick.converting.one": "Đang chuyển {count} file sang {target}…",
  "quick.converting.other": "Đang chuyển {count} file sang {target}…",
  "quick.finished": "Hoàn tất",
  "quick.succeeded": "{count} thành công",
  "quick.autoClose": "Tự đóng khi xong",
  "quick.openFolder": "Mở thư mục",
  "quick.close": "Đóng",

  // ----- File dialogs -----
  "dialog.pickFiles": "Chọn file cần chuyển đổi",
  "dialog.pickTemplate": "Chọn file Word mẫu (lấy font, kiểu tiêu đề, lề…)",
  "dialog.pickFolder": "Chọn thư mục lưu kết quả",
};
