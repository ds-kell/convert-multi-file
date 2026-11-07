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
- [x] Kiểm tra `cargo --version`, `cargo tauri --version` (đã chạy được khi build)

---

## Phase 1 – Khung project + chuyển ảnh cơ bản ⭐

### 1.1 Workspace
- [x] `git init`, tạo `.gitignore` (target/, node_modules/, dist/)
- [x] `Cargo.toml` workspace với các member `crates/core`, `crates/cli`
- [x] `rustfmt.toml`, bật `clippy` (thêm phần `[workspace.lints]`)
- [x] `README.md`: cách build và chạy

### 1.2 Core – nền tảng
- [x] enum `Format` (Png, Jpg, Bmp, Gif, Tiff, Ico…) + `from_extension()` + `extension()`
- [x] Nhận biết định dạng theo magic bytes (crate `infer`); nếu đuôi file sai thì ưu tiên magic bytes
- [x] struct `Job { input, target, output_dir, options }`
- [x] trait `Converter` (`id`, `inputs`, `outputs`, `is_available`, `convert`)
- [x] `Registry`: đăng ký converter, `targets_for(path)`, `find(input, target)`
- [x] `ConvertError` (dùng `thiserror`): Unsupported, ToolMissing, Io, Decode, Encode
- [x] Hàm đặt tên file đầu ra: lưu cạnh file gốc, trùng thì `name (1).ext`
- [x] trait `Progress` (callback 0–100%), kèm một bản rỗng `NoProgress`

### 1.3 Converter ảnh (crate `image`)
- [x] `ImageConverter`: png ↔ jpg ↔ bmp ↔ gif ↔ tiff ↔ ico
- [x] Chuyển png có nền trong suốt sang jpg/bmp → điền nền trắng
- [x] Chuyển sang ico → resize về tối đa 256px, giữ tỉ lệ
- [x] gif động → chỉ lấy frame đầu (ghi chú giới hạn này)

### 1.4 CLI (`clap`)
- [x] `convertapp-cli formats <file>` → in ra các định dạng đích
- [x] `convertapp-cli convert --to <fmt> [--out <dir>] <files...>`
- [x] Mã thoát (exit code) khác 0 nếu có file lỗi, in lỗi rõ ràng từng file
- [x] Log bằng `tracing`, bật chi tiết với `-v`

### 1.5 Test
- [x] Ảnh mẫu: sinh lúc chạy test từ cùng một bộ pixel (thay cho file nhị phân trong `fixtures/`), để so sánh được kích thước và màu chính xác
- [x] Unit test: `Format::from_extension`, đặt tên file trùng, Registry
- [x] Integration test: convert mỗi cặp định dạng, mở lại file kết quả xem có đúng định dạng không

### ✅ Nghiệm thu
- [x] `cargo test` pass, `cargo clippy` không có cảnh báo
- [x] Chạy `cargo run -p convertapp-cli -- convert --to jpg a.png` ra `a.jpg` mở xem được

---

## Phase 2 – Markdown ↔ HTML ⭐

- [x] Thêm `Format::Md`, `Format::Html`
- [x] `MarkdownConverter` md → html bằng `comrak` (bật GFM: bảng, checklist, gạch ngang chữ, tự nhận link)
- [x] Template HTML: `<head>` có UTF-8 và CSS mặc định nhúng sẵn (kiểu github-markdown), title lấy từ heading đầu tiên
- [x] Tùy chọn: chọn CSS sáng/tối hoặc không dùng CSS
- [x] Đường dẫn ảnh tương đối giữ nguyên (file html nằm cạnh file md)
- [x] `HtmlToMarkdownConverter` html → md bằng `htmd`
- [x] Nhận diện encoding của file html (`encoding_rs`, đọc thẻ meta charset)
- [x] Fixtures: md có bảng, code block, ảnh, tiếng Việt; html thực tế (lưu từ một trang web)
- [x] Test: md → html → md giữ được nội dung chính

### ✅ Nghiệm thu
- [x] CLI convert `README.md` → html (đã kiểm tra nội dung bằng test)
- [ ] Mở file html bằng trình duyệt, xem bằng mắt: hiển thị đẹp, tiếng Việt không lỗi font

---

## Phase 3 – Giao diện Tauri tối thiểu ⭐⭐

### 3.1 Khởi tạo
- [x] Dựng tay khung Tauri + React + TS + Vite trong `app/` (frontend ở `app/src`), thêm `app/src-tauri` vào workspace
- [x] `src-tauri` phụ thuộc vào `convertapp-core`
- [x] Cấu hình tên app, identifier, icon tạm

### 3.2 Backend commands
- [x] `inspect_files(paths)` → định dạng từng file, lỗi từng file, các định dạng đích chung
- [x] `convert_files(paths, target, settings, channel)` chạy ở `spawn_blocking`
- [x] Gửi event qua Tauri `Channel`: started / progress / done / skipped / failed / finished
- [x] "Mở thư mục": dùng `revealItemInDir` của `tauri-plugin-opener` ngay từ frontend

### 3.3 Frontend
- [x] Vùng kéo-thả file (sự kiện drag-drop của Tauri) + nút "Chọn file" (`tauri-plugin-dialog`)
- [x] Danh sách file: tên, định dạng, trạng thái, nút xóa
- [x] Danh sách định dạng đích (nút hoặc dropdown), disable nếu không file nào hỗ trợ
- [x] Nút Convert, thanh tiến trình, báo lỗi cạnh từng file
- [x] Nút "Mở thư mục" khi xong
- [x] Giao diện sáng/tối theo hệ thống

### ✅ Nghiệm thu
- [ ] `cargo tauri dev`: kéo 3 ảnh + 1 md vào → chỉ hiện định dạng chung (hoặc báo không có) → convert ảnh thành công

---

## Phase 4 – Menu chuột phải trên Windows ⭐⭐

### 4.1 Tham số dòng lệnh cho app
- [x] `ConvertApp.exe --to <fmt> <files...>` → chế độ "quick convert" (không mở cửa sổ chính). Dùng `clap` thay cho `tauri-plugin-cli` (cùng parser cho tiến trình được chuyển tiếp)
- [x] Cửa sổ nhỏ quick convert: tiến trình + lỗi từng file + tự đóng khi xong (checkbox, nhớ lựa chọn)

### 4.2 Gom nhiều file
- [x] `tauri-plugin-single-instance`: tiến trình mở sau gửi tham số cho tiến trình đầu tiên
- [x] Gom các file nhận được, đợi 300 ms yên lặng → một lượt, nhóm theo định dạng đích, bỏ trùng
- [x] Test: giả lập 20 tiến trình cùng lúc → 1 cửa sổ, 20/20 file, tự đóng

### 4.3 Crate `shell` (dùng `winreg`)
- [x] `register(registry)`: mỗi đuôi file ghi `HKCU\Software\Classes\SystemFileAssociations\.<ext>\shell\ConvertApp`
- [x] Menu con: `MUIVerb="Convert to"` + `ExtendedSubCommandsKey` (thay cho `SubCommands`, vì `SubCommands` cần quyền admin) → mỗi định dạng đích một mục
- [x] Icon cho menu
- [x] `unregister()`: xóa sạch
- [x] Gọi `SHChangeNotify` để Explorer cập nhật ngay

### 4.4 Tích hợp
- [x] `convertapp.exe --register` / `--unregister`
- [x] Ô "Menu chuột phải" trên header app: bật/tắt

### ✅ Nghiệm thu
- [x] Đã đăng ký thật cho `target\release\convertapp.exe` (18 đuôi file), đã kiểm tra registry
- [ ] Bạn tự thử: chuột phải 5 file png → Show more options → Convert to ▸ JPG → 1 cửa sổ, ra đủ 5 file
- [ ] Bạn tự thử: bỏ tick "Menu chuột phải" → menu biến mất

---

## Phase 5 – Ảnh nâng cao ⭐⭐

- [x] **webp** encode lossy (crate `webp`/libwebp) + lossless, giữ trong suốt; decode bằng `image`
- [x] **svg** → png/jpg/webp/bmp/gif/tiff/ico bằng `resvg`, font hệ thống nạp 1 lần; render thẳng ở kích thước yêu cầu (phóng to vẫn nét). Không có svg → pdf (sẽ bị raster hoá)
- [x] **ảnh → pdf** bằng `pdf-writer` (thay cho `printpdf`, API ổn định hơn): 1 ảnh/trang, A4 (tự xoay ngang, không phóng to) hoặc theo kích thước ảnh
- [x] Gộp nhiều ảnh thành 1 pdf: `--merge` ở CLI + checkbox "Gộp N ảnh" ở UI
- [x] `ImageOptions`: `quality` (JPG/WebP/ảnh trong PDF), `webp_lossless`, `resize` (giữ nguyên / giới hạn px / %), `pdf_page`. Metadata (EXIF, GPS…) luôn bị loại bỏ
- [x] Xoay ảnh đúng theo EXIF orientation trước khi convert
- [x] UI: panel tùy chọn hiện theo định dạng đích
- [x] Test cho webp, svg, pdf, EXIF, resize (12 test mới)

### ✅ Nghiệm thu
- [x] Test tự động: JPEG 40×20 có EXIF orientation=6 → webp q=80 → ra 20×40 (đúng chiều); webp lossy nhỏ hơn lossless, q30 < q90
- [x] Chạy app thật (qua CDP): thả 2 PNG → PDF → tick gộp → ra 1 file `a.pdf`
- [ ] Bạn tự thử với ảnh chụp điện thoại thật → webp q=80

---

## Phase 6 – Dùng Pandoc ⭐⭐

### 6.1 Hạ tầng tool ngoài
- [x] Module `tools`: tìm exe theo biến môi trường (`CONVERTAPP_PANDOC`, `CONVERTAPP_TYPST`) → PATH → thư mục cài quen thuộc
- [ ] Đường dẫn tool tự đặt trong **Settings của app** (hiện mới có qua biến môi trường; cần chỗ lưu settings, để làm cùng Phase 7/8)
- [x] `tools::run`: lấy stderr, quá thời gian thì kill, ẩn cửa sổ console (`CREATE_NO_WINDOW`)
- [x] Converter báo `Unavailable` kèm lệnh cài; `Registry::locked_targets`; UI hiện chip 🔒 + lý do

### 6.2 Converter
- [x] md → docx, html → docx
- [x] docx → md (ảnh tách ra `name_files/`), docx → html (1 file, ảnh nhúng sẵn)
- [x] md → pdf qua Typst. **Không** dùng `--pdf-engine=typst` vì Pandoc truyền đường dẫn ảnh `C:\…` mà Typst từ chối trên Windows → làm 2 bước: Pandoc xuất `.typ`, Typst compile với `--root`
- [x] Font PDF: Segoe UI (Windows) để hiện đủ dấu tiếng Việt
- [x] Tùy chọn file docx mẫu: `--reference-doc` ở CLI, nút "File Word mẫu" ở app

### 6.3 Test
- [x] Test tự bỏ qua (in "skipped") nếu thiếu pandoc/typst
- [x] File docx sinh lúc chạy test từ md có bảng + ảnh + tiếng Việt, kiểm tra bên trong file (XML) — thay cho fixture nhị phân

### ✅ Nghiệm thu
- [x] md tiếng Việt → docx, mở bằng **Microsoft Word thật** (qua COM): 1 bảng, Heading 1, tiếng Việt đúng, 1 ảnh
- [x] md → pdf: render thử thấy tiếng Việt hiển thị đúng
- [x] Giả lập máy thiếu Pandoc: PDF/DOCX hiện 🔒 kèm lệnh cài

---

## Phase 7 – HTML → PDF/PNG qua trình duyệt ⭐⭐⭐

- [x] Dò Edge/Chrome: registry `App Paths`, thư mục cài mặc định (+ macOS/Linux); tự đặt đường dẫn bằng `CONVERTAPP_BROWSER`
- [x] Điều khiển trình duyệt ẩn qua CDP bằng **client đồng bộ tự viết** (dùng `tungstenite`) thay cho `chromiumoxide`: không cần tokio, code nhỏ, hợp với engine đồng bộ
- [x] Mỗi file một trình duyệt riêng (profile tạm, ~1,3 s/file) thay vì dùng chung cả lượt: không lẫn cookie/trạng thái giữa các file, không bao giờ để sót tiến trình chạy ngầm. Có test kiểm tra không còn tiến trình sót
- [x] html → pdf: khổ giấy (A4/A3/A5/Letter/Legal), lề, in nền, ngang/dọc; tiêu đề thành bookmark PDF
- [x] html → png: chụp toàn trang (tối đa 16384 px cao), chọn chiều rộng cửa sổ
- [x] md → pdf/png: md → html (Phase 2) → trình duyệt. Ưu tiên trước Pandoc + Typst (chỉ cần Edge là đủ)
- [x] Chờ sự kiện `load` + font web sẵn sàng, có timeout
- [x] Không có trình duyệt → `Unavailable` kèm hướng dẫn (hiện 🔒 trong app)

### ✅ Nghiệm thu
- [x] Test với Edge thật: A4 dọc 595×842 pt, Letter ngang 792×612 pt; PNG 800 px rộng, cao ≥ 3000 px, ảnh tương đối tải được
- [x] App thật: html → pdf thành công; menu chuột phải (bản release) html → png ra đúng trang
- [ ] Bạn tự so sánh PDF với khi in thủ công từ Edge (Ctrl+P → Lưu thành PDF)

---

## Phase 8 – Office ⭐⭐⭐

### 8.1 LibreOffice
- [x] Cài LibreOffice 26.8 vào `E:\DevTools\LibreOffice`; dò `soffice` qua biến `CONVERTAPP_SOFFICE`, PATH, Program Files, **App Paths** (registry)
- [x] Dùng `soffice.com` (đợi xong và trả mã lỗi), không dùng `soffice.exe` (trả về ngay)
- [x] `--convert-to` vào thư mục tạm trên cùng ổ đích rồi đổi tên; kiểm tra file ra thật (LibreOffice trả 0 kể cả khi lỗi)
- [x] Mỗi job một profile riêng → chạy song song được (~3–5 s/file)
- [x] Văn bản: docx/doc/odt/rtf → pdf, docx, odt, rtf. **Không có txt**: LibreOffice 26.8 treo khi xuất text tài liệu có bảng → txt do Word hoặc Pandoc làm
- [x] Bảng tính: xlsx/xls/ods/csv → pdf, xlsx, ods, csv (CSV UTF-8, dấu phẩy; xuất sheet đầu)
- [x] Trình chiếu: pptx/ppt/odp → pdf, pptx, odp
- [ ] Bỏ: docx → html bằng Office (tạo thư mục ảnh tên rối) → đã có Pandoc làm (1 file)

### 8.2 MS Office (chỉ Windows)
- [x] Script PowerShell nhúng vào app (`assets/office.ps1`): Word/Excel/PowerPoint COM → `ExportAsFixedFormat` / `SaveAs`
- [x] Luôn đóng tài liệu + `Quit()` + giải phóng COM trong `finally`; ghi PID Office để kill nếu treo; không bao giờ tắt PowerPoint người dùng đang mở; 1 job Office tại một thời điểm
- [x] Mở chỉ đọc với mật khẩu giả → file có mật khẩu báo lỗi thay vì hiện hộp thoại không ai bấm được
- [x] Excel đọc CSV đúng UTF-8 + dấu phẩy (mở bản sao `.txt` bằng `OpenText`, vì Excel bỏ qua tùy chọn với đuôi `.csv`)
- [x] Phát hiện từng app qua ProgID trong registry

### 8.3 Chọn engine + ảnh slide
- [x] Thứ tự: MS Office → LibreOffice; chọn được ở CLI (`--office`) và app (ô "Bộ Office")
- [x] `tools::run` kill **cả cây tiến trình** khi quá giờ (soffice.com → soffice.bin)
- [x] ~~pptx → png từng slide~~: **bỏ** (quyết định 2026-11-08: app giữ nguyên tắc n file vào → n file ra)

### ✅ Nghiệm thu
- [x] Test docx (bảng + tiếng Việt), rtf, csv → xlsx → pdf/csv, pptx → pdf/odp: chạy **với cả MS Office và LibreOffice**, đều pass
- [x] App thật: thả docx + xlsx + pptx → PDF → 3 thành công, không sót tiến trình Office
- [ ] Bạn tự mở vài PDF để so sánh chất lượng giữa 2 engine

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

## Đa ngôn ngữ (English / Tiếng Việt) ✅

- [x] Bộ i18n nhỏ tự viết, có kiểu chặt: thiếu bản dịch = lỗi biên dịch; tham số `{name}`; số nhiều tiếng Anh
- [x] Chọn ngôn ngữ trên header; mặc định theo Windows; nhớ lựa chọn
- [x] Toàn bộ giao diện (cửa sổ chính, cửa sổ chuyển nhanh, hộp thoại chọn file) qua `t()`
- [x] Backend trả mã lỗi thay vì câu tiếng Việt cứng
- [x] Tiêu đề menu chuột phải theo ngôn ngữ; `--register` theo ngôn ngữ Windows
- [x] Kiểm tra trên app thật cả 2 ngôn ngữ (CDP + ảnh chụp)
- [ ] Lỗi kỹ thuật từ công cụ ngoài vẫn tiếng Anh (có chủ đích); CLI vẫn tiếng Anh

---

## Gộp file (n vào → 1 ra) ✅

- [x] Gộp ảnh + PDF thành 1 PDF theo đúng thứ tự (lopdf; giữ thuộc tính trang kế thừa; giữ thứ tự trang)
- [x] Nối ảnh dọc/ngang thành 1 ảnh: khoảng cách, cùng chiều rộng/cao (không phóng to), giới hạn kích thước
- [x] `merge_targets()`: app chỉ hiện lựa chọn gộp khi hợp lệ; nhiều PDF → chỉ có "gộp"
- [x] CLI `--merge --direction --gap --keep-sizes`; app có checkbox + tùy chọn nối, EN/VI
- [x] Test: thứ tự trang khi trộn PDF + ảnh, PDF do Chrome tạo, nối dọc/ngang, SVG + ảnh thường
- [ ] Không mang theo bookmark/form của PDF gốc (chỉ nội dung trang); PDF có mật khẩu bị từ chối

---

## Backlog (chưa xếp phase)
- ~~pptx/pdf → png từng trang~~: **bỏ**. Nguyên tắc của app: n file vào → n file ra, không tách 1 file thành nhiều file
- [ ] Đường dẫn tool tự đặt trong Settings của app (Pandoc/Typst/LibreOffice/trình duyệt) + lưu settings giữa các lần mở
- [ ] HEIC/AVIF
- [ ] PDF → ảnh / PDF → docx
- [ ] Đặt preset convert hay dùng (ví dụ "Ảnh web: webp q80 max 1920px")
- [ ] Theo dõi thư mục, tự động convert file mới
- [ ] Đa ngôn ngữ giao diện (vi/en)
