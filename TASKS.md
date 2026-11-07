# ConvertApp – Danh sách task

Đánh dấu `[x]` khi xong. Mỗi phase có mục **✅ Nghiệm thu**: làm đủ các bước trong đó mới chuyển sang phase tiếp theo.
Chi tiết thiết kế xem [PLAN.md](PLAN.md).

---

## Phase 0 – Cài tool ✅
- [x] winget vào PATH
- [x] MSVC Build Tools (VCTools) → ổ E
- [x] Rust stable → `E:\DevTools`
- [x] Tauri CLI 2.x
- [x] Pandoc
- [x] Extension VS Code: rust-analyzer, tauri, lldb, toml
- [ ] Mở lại VS Code và terminal, chạy `cargo --version` và `cargo tauri --version` xem có ra phiên bản không

---

## Phase 1 – Khung project + chuyển ảnh cơ bản ⭐

### 1.1 Workspace
- [ ] `git init`, tạo `.gitignore` (target/, node_modules/, dist/)
- [ ] `Cargo.toml` workspace với các member `crates/core`, `crates/cli`
- [ ] `rustfmt.toml`, bật `clippy` (thêm phần `[workspace.lints]`)
- [ ] `README.md`: cách build và chạy

### 1.2 Core – nền tảng
- [ ] enum `Format` (Png, Jpg, Bmp, Gif, Tiff, Ico…) + `from_extension()` + `extension()`
- [ ] Nhận biết định dạng theo magic bytes (crate `infer`); nếu đuôi file sai thì ưu tiên magic bytes
- [ ] struct `Job { input, target, output_dir, options }`
- [ ] trait `Converter` (`id`, `inputs`, `outputs`, `is_available`, `convert`)
- [ ] `Registry`: đăng ký converter, `targets_for(path)`, `find(input, target)`
- [ ] `ConvertError` (dùng `thiserror`): Unsupported, ToolMissing, Io, Decode, Encode
- [ ] Hàm đặt tên file đầu ra: lưu cạnh file gốc, trùng thì `name (1).ext`
- [ ] trait `Progress` (callback 0–100%), kèm một bản rỗng `NoProgress`

### 1.3 Converter ảnh (crate `image`)
- [ ] `ImageConverter`: png ↔ jpg ↔ bmp ↔ gif ↔ tiff ↔ ico
- [ ] Chuyển png có nền trong suốt sang jpg/bmp → điền nền trắng
- [ ] Chuyển sang ico → resize về tối đa 256px, giữ tỉ lệ
- [ ] gif động → chỉ lấy frame đầu (ghi chú giới hạn này)

### 1.4 CLI (`clap`)
- [ ] `convertapp-cli formats <file>` → in ra các định dạng đích
- [ ] `convertapp-cli convert --to <fmt> [--out <dir>] <files...>`
- [ ] Mã thoát (exit code) khác 0 nếu có file lỗi, in lỗi rõ ràng từng file
- [ ] Log bằng `tracing`, bật chi tiết với `-v`

### 1.5 Test
- [ ] `crates/core/tests/fixtures/`: ảnh mẫu nhỏ (png trong suốt, jpg, bmp, gif, tiff)
- [ ] Unit test: `Format::from_extension`, đặt tên file trùng, Registry
- [ ] Integration test: convert mỗi cặp định dạng, mở lại file kết quả xem có đúng định dạng không

### ✅ Nghiệm thu
- [ ] `cargo test` pass, `cargo clippy` không có cảnh báo
- [ ] Chạy `cargo run -p convertapp-cli -- convert --to jpg a.png` ra `a.jpg` mở xem được

---

## Phase 2 – Markdown ↔ HTML ⭐

- [ ] Thêm `Format::Md`, `Format::Html`
- [ ] `MarkdownConverter` md → html bằng `comrak` (bật GFM: bảng, checklist, gạch ngang chữ, tự nhận link)
- [ ] Template HTML: `<head>` có UTF-8 và CSS mặc định nhúng sẵn (kiểu github-markdown), title lấy từ heading đầu tiên
- [ ] Tùy chọn: chọn CSS sáng/tối hoặc không dùng CSS
- [ ] Đường dẫn ảnh tương đối giữ nguyên (file html nằm cạnh file md)
- [ ] `HtmlToMarkdownConverter` html → md bằng `htmd`
- [ ] Nhận diện encoding của file html (`encoding_rs`, đọc thẻ meta charset)
- [ ] Fixtures: md có bảng, code block, ảnh, tiếng Việt; html thực tế (lưu từ một trang web)
- [ ] Test: md → html → md giữ được nội dung chính

### ✅ Nghiệm thu
- [ ] CLI convert `README.md` → html, mở bằng trình duyệt thấy hiển thị đẹp, tiếng Việt không lỗi font

---

## Phase 3 – Giao diện Tauri tối thiểu ⭐⭐

### 3.1 Khởi tạo
- [ ] `cargo create-tauri-app` (React + TS + Vite) vào thư mục `app/`, thêm `app/src-tauri` vào workspace
- [ ] `src-tauri` phụ thuộc vào `convertapp-core`
- [ ] Cấu hình tên app, identifier, icon tạm

### 3.2 Backend commands
- [ ] `get_targets(paths) -> Vec<Format>` (chỉ lấy định dạng mà mọi file đã chọn đều hỗ trợ)
- [ ] `convert(paths, target, options) -> job_id`, chạy ở `spawn_blocking`
- [ ] Gửi event `job-progress` / `job-done` / `job-error` theo từng file
- [ ] `open_folder(path)` (`tauri-plugin-opener`)

### 3.3 Frontend
- [ ] Vùng kéo-thả file (sự kiện drag-drop của Tauri) + nút "Chọn file" (`tauri-plugin-dialog`)
- [ ] Danh sách file: tên, định dạng, trạng thái, nút xóa
- [ ] Danh sách định dạng đích (nút hoặc dropdown), disable nếu không file nào hỗ trợ
- [ ] Nút Convert, thanh tiến trình, báo lỗi cạnh từng file
- [ ] Nút "Mở thư mục" khi xong
- [ ] Giao diện sáng/tối theo hệ thống

### ✅ Nghiệm thu
- [ ] `cargo tauri dev`: kéo 3 ảnh + 1 md vào → chỉ hiện định dạng chung (hoặc báo không có) → convert ảnh thành công

---

## Phase 4 – Menu chuột phải trên Windows ⭐⭐

### 4.1 Tham số dòng lệnh cho app
- [ ] `tauri-plugin-cli`: `ConvertApp.exe --to <fmt> <files...>` → chế độ "quick convert" (không mở cửa sổ chính)
- [ ] Cửa sổ nhỏ chế độ quick convert: tiến trình + danh sách lỗi + tự đóng khi xong (có tùy chọn)

### 4.2 Gom nhiều file
- [ ] `tauri-plugin-single-instance`: tiến trình mở sau gửi tham số cho tiến trình đầu tiên
- [ ] Gom các file nhận được trong ~300ms → xử lý một lượt
- [ ] Test: chọn 20 file → chỉ 1 cửa sổ

### 4.3 Crate `shell` (dùng `winreg`)
- [ ] `register(exe_path, registry)`: với mỗi đuôi file ghi `HKCU\Software\Classes\SystemFileAssociations\.<ext>\shell\ConvertApp`
- [ ] Menu con: `MUIVerb="Convert to"`, `SubCommands` → mỗi định dạng đích một mục
- [ ] Icon cho menu
- [ ] `unregister()`: xóa sạch
- [ ] Gọi `SHChangeNotify` để Explorer cập nhật ngay

### 4.4 Tích hợp
- [ ] CLI: `--register` / `--unregister` để test lúc dev
- [ ] Trang Settings trong app: bật/tắt menu chuột phải

### ✅ Nghiệm thu
- [ ] Chuột phải 5 file png → Show more options → Convert to ▸ JPG → 1 cửa sổ, ra đủ 5 file
- [ ] Gỡ đăng ký → menu biến mất

---

## Phase 5 – Ảnh nâng cao ⭐⭐

- [ ] **webp** encode lossy (crate `webp`), decode bằng `image`
- [ ] **svg** → png/jpg bằng `resvg` (+ `usvg` nạp font hệ thống), chọn kích thước/scale
- [ ] **ảnh → pdf** (`printpdf`): 1 ảnh/trang, khổ A4 hoặc theo kích thước ảnh
- [ ] Gộp nhiều ảnh thành 1 pdf (option `merge` ở CLI + checkbox ở UI)
- [ ] `ImageOptions`: chất lượng (JPG/WebP), resize (max width/height, %), xóa EXIF
- [ ] Xoay ảnh đúng theo EXIF orientation trước khi convert
- [ ] UI: panel tùy chọn hiện theo định dạng đích
- [ ] Fixtures + test cho webp, svg, pdf

### ✅ Nghiệm thu
- [ ] Convert ảnh chụp điện thoại (bị xoay theo EXIF) → webp q=80 → hiển thị đúng chiều, dung lượng giảm

---

## Phase 6 – Dùng Pandoc ⭐⭐

### 6.1 Hạ tầng tool ngoài
- [ ] Module `tools`: tìm file exe theo PATH + các thư mục cài đặt quen thuộc + đường dẫn người dùng tự đặt trong Settings
- [ ] `run_tool(cmd, args, timeout)`: lấy stderr, chạy quá lâu thì dừng, ẩn cửa sổ console (`CREATE_NO_WINDOW`)
- [ ] `is_available()` trả lý do + link hướng dẫn cài; UI hiện định dạng bị khóa kèm lý do

### 6.2 Converter
- [ ] md → docx, html → docx
- [ ] docx → md, docx → html (tách ảnh ra thư mục `name_files/`)
- [ ] md → pdf qua `--pdf-engine=typst` (kiểm tra có typst trên máy không)
- [ ] Tùy chọn: file docx mẫu (`--reference-doc`) để định dạng theo style riêng

### 6.3 Test
- [ ] Test có `#[ignore]` hoặc tự bỏ qua nếu thiếu pandoc/typst
- [ ] Fixture docx có bảng + ảnh + tiếng Việt

### ✅ Nghiệm thu
- [ ] md tiếng Việt → docx mở bằng Word đúng font, đúng bảng

---

## Phase 7 – HTML → PDF/PNG qua trình duyệt ⭐⭐⭐

- [ ] Dò Edge/Chrome: registry `App Paths`, các thư mục cài mặc định; cho phép tự đặt đường dẫn
- [ ] `chromiumoxide`: chạy trình duyệt ẩn, dùng chung 1 trình duyệt cho cả lượt convert
- [ ] html → pdf: khổ giấy, lề, in nền, ngang/dọc
- [ ] html → png: chụp toàn trang, chọn chiều rộng viewport
- [ ] md → pdf bằng cách 2: md → html (Phase 2) → Chromium
- [ ] Chờ tải xong ảnh/font trước khi in (sự kiện `load` + timeout)
- [ ] Không có trình duyệt → converter báo `Unavailable`

### ✅ Nghiệm thu
- [ ] Trang HTML có CSS + ảnh → PDF giống như in thủ công từ Edge

---

## Phase 8 – Office ⭐⭐⭐

### 8.1 LibreOffice
- [ ] Dò `soffice` (đường dẫn cài đặt của Windows/macOS/Linux)
- [ ] `soffice --headless --convert-to <fmt> --outdir <tmp>` rồi chuyển file kết quả vào thư mục đích
- [ ] Mỗi job dùng một profile riêng (`-env:UserInstallation=file:///…`) để chạy song song không lỗi
- [ ] Word: docx/doc/odt/rtf → pdf, docx, odt, txt, html
- [ ] Excel: xlsx/xls/ods → pdf, xlsx, csv (chọn sheet)
- [ ] PowerPoint: pptx/ppt/odp → pdf

### 8.2 MS Office (chỉ Windows)
- [ ] Script PowerShell nhúng vào app: Word/Excel/PowerPoint COM → `ExportAsFixedFormat` / `SaveAs`
- [ ] Đảm bảo `Quit()` + giải phóng COM kể cả khi lỗi; chạy quá lâu thì kill
- [ ] Phát hiện có Office không (registry ClickToRun / ProgID)

### 8.3 Chọn engine + ảnh slide
- [ ] Thứ tự ưu tiên: MS Office → LibreOffice; cho phép chọn trong Settings
- [ ] pptx/pdf → png từng trang bằng `pdfium-render` (đóng gói kèm `pdfium.dll`)

### ✅ Nghiệm thu
- [ ] docx/xlsx/pptx mẫu → pdf bằng cả 2 engine, so sánh kết quả

---

## Phase 9 – Đóng gói Windows ⭐⭐

- [ ] Icon thật (`cargo tauri icon`), thông tin version/publisher
- [ ] Bundler NSIS, cài cho từng user (`installMode: currentUser`)
- [ ] NSIS hook: đăng ký menu chuột phải khi cài, gỡ khi uninstall
- [ ] Đóng gói kèm `pdfium.dll`; Pandoc/LibreOffice **không** đóng kèm (hiện hướng dẫn cài)
- [ ] (Tùy chọn) `tauri-plugin-updater` + chữ ký cho bản cập nhật
- [ ] Test trên máy sạch (Windows Sandbox): cài → dùng → gỡ không để lại gì

### ✅ Nghiệm thu
- [ ] File `ConvertApp_x.y.z_x64-setup.exe` cài/gỡ sạch, menu chuột phải hoạt động sau khi cài

---

## Phase 10 – Đa nền tảng + menu Win 11 ⭐⭐⭐⭐

### 10.1 macOS
- [ ] Build `.app` / `.dmg`
- [ ] Quick Action (Shortcuts/Automator) gọi CLI cho từng định dạng
- [ ] (Nâng cao) Finder Sync Extension bằng Swift, ký số + notarize

### 10.2 Linux
- [ ] Build `.deb` / AppImage
- [ ] Sinh file Nautilus script, Dolphin service menu, Nemo action
- [ ] Kiểm tra trên Ubuntu (GNOME) + Kubuntu (KDE)

### 10.3 Menu chính Win 11
- [ ] `IExplorerCommand` (crate `windows`, DLL COM) có menu con động
- [ ] Sparse package (AppxManifest) + chứng chỉ ký số
- [ ] Giữ lại menu registry để dùng trên Win 10

### 10.4 CI
- [ ] GitHub Actions: test cả 3 hệ điều hành, build bằng `tauri-action`, tạo release

---

## Backlog (chưa xếp phase)
- [ ] HEIC/AVIF
- [ ] PDF → ảnh / PDF → docx
- [ ] Đặt preset convert hay dùng (ví dụ "Ảnh web: webp q80 max 1920px")
- [ ] Theo dõi thư mục, tự động convert file mới
- [ ] Đa ngôn ngữ giao diện (vi/en)
