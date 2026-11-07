# ConvertApp – Kế hoạch (Tauri v2 + Rust)

App desktop chạy được trên Windows, macOS và Linux để chuyển đổi file (ảnh, Markdown, HTML, Office). Có 2 cách dùng:
1. Chuột phải vào file trong trình quản lý file → **Convert to ▸ …**
2. Mở app như bình thường, kéo-thả file vào

**Nguyên tắc:** làm phần dễ, thuần Rust trước → test được ngay → mỗi phase thêm một nhóm định dạng. Hết mỗi phase app phải **chạy được và có test**.

---

## Công nghệ

| Phần | Chọn |
|---|---|
| Ngôn ngữ backend | Rust (edition 2021, bản stable) |
| Khung app desktop | Tauri v2 |
| Giao diện | React + TypeScript + Vite (đổi sang Svelte/Vue được, không ảnh hưởng backend) |
| Gọi từ dòng lệnh | `clap` (CLI riêng) + `tauri-plugin-cli` (trong app) |
| Gom nhiều file khi chọn nhiều | `tauri-plugin-single-instance` |
| Đóng gói | Bộ bundler có sẵn của Tauri (NSIS/MSI trên Windows, .dmg trên macOS, .deb/AppImage trên Linux), không cần Inno Setup |

---

## Phase 0 – Cài tool

✅ **Đã xong (2026-11-07).** Ưu tiên cài vào ổ E.

| Tool | Phiên bản | Vị trí |
|---|---|---|
| Git, VS Code, Node 25, WebView2 | có sẵn | |
| winget | 1.29 | đã thêm vào PATH |
| MSVC Build Tools 2022 (VCTools) | 14.44 | `E:\Program Files\Microsoft Visual Studio\2022\BuildTools`, cache ở `E:\DevTools\VSPackages`, `E:\DevTools\VSShared` |
| Windows SDK | 10.0.26100 | `C:\Program Files (x86)\Windows Kits` (bắt buộc nằm ở C) |
| Rust stable | 1.99.1 | `RUSTUP_HOME=E:\DevTools\rustup`, `CARGO_HOME=E:\DevTools\cargo` (biến môi trường user) |
| Tauri CLI | 2.12.4 | `E:\DevTools\cargo\bin\cargo-tauri.exe` |
| Pandoc | 3.13 | `%LOCALAPPDATA%\Pandoc` (cần từ Phase 6) |
| VS Code extensions | | rust-analyzer, tauri-vscode, vscode-lldb, even-better-toml |
| .NET 8 SDK | đã cài, **không cần nữa** | Gỡ nếu muốn: `winget uninstall Microsoft.DotNet.SDK.8` |

Kiểm tra (mở terminal mới):
```powershell
rustc --version; cargo --version; cargo tauri --version
```

---

## Cấu trúc project

```
convert_file/
├─ Cargo.toml                 (workspace)
├─ crates/
│  ├─ core/                   ← trait Converter, Registry, các converter, phát hiện tool ngoài
│  │  └─ tests/fixtures/      ← file mẫu để test (png, md, html, docx…)
│  ├─ cli/                    ← binary `convertapp-cli` để test nhanh không cần UI
│  └─ shell/                  ← đăng ký menu chuột phải (win registry / mac / linux)
└─ app/
   ├─ src-tauri/              ← backend Tauri, gọi crates/core
   └─ ui/                     ← React + Vite
```

### Thiết kế core
```rust
pub trait Converter: Send + Sync {
    fn id(&self) -> &'static str;
    fn inputs(&self) -> &[Format];                 // định dạng vào
    fn outputs(&self, input: Format) -> Vec<Format>; // định dạng ra
    fn is_available(&self) -> Availability;        // ví dụ: thiếu pandoc → Unavailable("cần cài Pandoc")
    fn convert(&self, job: &Job, progress: &dyn Progress) -> Result<PathBuf>;
}
```
- `Registry::targets_for(path)` → danh sách định dạng đích. **CLI, giao diện và menu chuột phải đều lấy từ đây.**
- Nhận biết định dạng theo đuôi file, kiểm tra thêm bằng magic bytes (crate `infer`)
- File đầu ra lưu cạnh file gốc, nếu trùng tên thì tự thêm `name (1).png`, `name (2).png`…
- Lỗi dùng `thiserror`, ghi log bằng `tracing`

---

## Lộ trình (dễ → khó)

### Phase 1 – Khung project + chuyển ảnh cơ bản ⭐ dễ
- Tạo workspace, `core` + `cli`
- Crate `image`: **png ↔ jpg ↔ bmp ↔ gif ↔ tiff ↔ ico** (ico tự resize về 256px)
- CLI: `convertapp-cli --to jpg a.png b.png` và `convertapp-cli formats a.png`
- **Xong khi:** `cargo test` pass với file mẫu, chạy CLI convert ra file đúng

### Phase 2 – Markdown ↔ HTML ⭐ dễ
- md → html: `comrak` (hỗ trợ GFM: bảng, checklist), bọc trong template HTML có sẵn CSS đẹp
- html → md: `htmd`
- **Xong khi:** test chuyển qua lại một file md mẫu có bảng, code block và ảnh

### Phase 3 – Giao diện Tauri tối thiểu ⭐⭐
- Kéo-thả hoặc chọn nhiều file → hiện danh sách định dạng đích (phần chung của các file đã chọn)
- Nút Convert, tiến trình từng file (Tauri event), báo lỗi theo từng file, nút "Mở thư mục"
- Chạy convert ở luồng nền (`spawn_blocking`) để không đơ giao diện
- **Xong khi:** `cargo tauri dev` mở app, kéo file ảnh hoặc md vào convert được

### Phase 4 – Menu chuột phải trên Windows ⭐⭐
- Crate `shell` dùng `winreg`, ghi vào `HKCU\Software\Classes\SystemFileAssociations\.<ext>\shell\ConvertApp` (không cần quyền admin)
- Menu con lấy từ Registry: "Convert to ▸ PNG / JPG / …", mỗi mục gọi `ConvertApp.exe --to <fmt> "%1"`
- `tauri-plugin-single-instance`: chọn 10 file thì gom về 1 cửa sổ tiến trình nhỏ (không mở cửa sổ chính)
- Trong Settings của app: bật/tắt menu chuột phải
- **Xong khi:** chuột phải vào 5 file png → Convert to JPG → 1 cửa sổ, ra đủ 5 file

### Phase 5 – Ảnh nâng cao ⭐⭐
- **webp** (crate `webp`, có nén lossy và chọn chất lượng)
- **svg → png/jpg** (`resvg`), chọn kích thước
- **ảnh → pdf** (`printpdf`); nhiều ảnh gộp thành 1 pdf
- Tùy chọn: chất lượng JPG/WebP, resize, giữ hoặc xóa EXIF
- **Xong khi:** test cho từng định dạng, tùy chọn hiện trong giao diện

### Phase 6 – Dùng Pandoc ⭐⭐
- Module `tools`: tìm `pandoc` trong PATH hoặc thư mục cài đặt; nếu không thấy thì converter báo `Unavailable` và giao diện hiện hướng dẫn cài
- md → **docx**, html → **docx**, docx → **md/html**
- md → **pdf** qua `pandoc --pdf-engine=typst` (cần cài thêm Typst: `winget install Typst.Typst`)
- **Xong khi:** test bỏ qua (skip) nếu máy không có pandoc, pass nếu có

### Phase 7 – HTML → PDF/PNG bằng trình duyệt chạy ngầm ⭐⭐⭐
- `chromiumoxide` điều khiển **Edge hoặc Chrome có sẵn trên máy** (tự dò đường dẫn)
- html → pdf (chọn khổ giấy, lề), html → png (chụp toàn trang)
- md → pdf thêm cách thứ 2 (md → html → Chromium), đẹp hơn Typst với nội dung web
- **Xong khi:** convert 1 trang HTML có CSS và ảnh ra PDF giống như khi in từ trình duyệt

### Phase 8 – Office ⭐⭐⭐
- **LibreOffice headless** (chạy được trên mọi hệ điều hành): `soffice --headless --convert-to pdf --outdir …`
  - docx/doc/odt/rtf → pdf, docx, odt, txt, html
  - xlsx/xls/ods → pdf, xlsx, csv
  - pptx/ppt/odp → pdf
- **Windows có MS Office:** script PowerShell nhỏ gọi COM (`ExportAsFixedFormat`), kết quả giống bản gốc hơn; đóng chương trình Office kể cả khi lỗi
- Thứ tự ưu tiên: MS Office → LibreOffice → báo `Unavailable`
- pptx → png từng slide: xuất ra pdf trước, rồi render từng trang (`pdfium-render`)
- **Xong khi:** convert docx/xlsx/pptx mẫu ra pdf bằng cả 2 cách

### Phase 9 – Đóng gói Windows ⭐⭐
- `cargo tauri build` → bộ cài NSIS cài vào thư mục người dùng
- Khi cài: đăng ký menu chuột phải; khi gỡ: xóa sạch registry
- Icon, tên app, version, tự kiểm tra bản cập nhật (`tauri-plugin-updater`, tùy chọn)

### Phase 10 – macOS, Linux và menu Win 11 ⭐⭐⭐⭐
- **macOS:** Quick Action (Shortcuts/Automator) gọi CLI → sau đó có thể làm Finder Sync Extension (Swift), ký số và notarize
- **Linux:** Nautilus script, Dolphin service menu, Nemo action (các file `.desktop` do crate `shell` sinh ra)
- **Win 11 menu chính:** `IExplorerCommand` bằng crate `windows` + sparse package + ký số
- CI build cả 3 hệ điều hành bằng GitHub Actions (`tauri-action`)

---

## Ma trận chuyển đổi (theo phase)

| Đầu vào | Đầu ra | Phase |
|---|---|---|
| png, jpg, bmp, gif, tiff, ico | qua lại giữa các định dạng này | 1 |
| md | html | 2 |
| html | md | 2 |
| webp | ↔ các định dạng ảnh | 5 |
| svg | png, jpg | 5 |
| ảnh | pdf | 5 |
| md | docx, pdf (Typst) | 6 |
| html | docx | 6 |
| docx | md, html | 6 |
| html, md | pdf, png (Chromium) | 7 |
| docx, doc, odt, rtf | pdf, docx, odt, txt | 8 |
| xlsx, xls, ods | pdf, xlsx, csv | 8 |
| pptx, ppt, odp | pdf, png | 8 |

## Rủi ro
- **Office**: phụ thuộc máy có MS Office hoặc LibreOffice → giao diện phải báo rõ nếu thiếu.
- **HTML → PDF**: cần Edge/Chrome; trên Linux có thể không có → tắt tính năng này.
- **Exe chưa ký số**: SmartScreen hoặc antivirus có thể cảnh báo → về lâu dài cần code-signing certificate.
- **HEIC/AVIF**: vướng codec và bản quyền → để sau Phase 10.
