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
