# ConvertApp

App desktop chuyển đổi file (ảnh, Markdown, HTML, Office), dùng được bằng chuột phải trong Explorer hoặc mở app như bình thường.
Viết bằng Rust + Tauri v2. Kế hoạch tổng thể xem [PLAN.md](PLAN.md), tiến độ xem [TASKS.md](TASKS.md).

## Cấu trúc

| Thư mục | Nội dung |
|---|---|
| `crates/core` | Engine chuyển đổi: `Format`, trait `Converter`, `Registry`, pipeline, các converter |
| `crates/cli` | `convertapp-cli`: dùng engine từ dòng lệnh, tiện để test nhanh |
| `crates/shell` | Đăng ký/gỡ menu chuột phải "Convert to" trong Explorer (registry HKCU) |
| `app/src-tauri` | Backend app desktop (Tauri v2): command `inspect_files`, `convert_files` |
| `app/src` | Giao diện React + TypeScript |

## Chạy app desktop

```powershell
cd app
npm install          # lần đầu
cargo tauri dev      # mở app, tự reload khi sửa code
cargo tauri build    # tạo bộ cài NSIS trong target/release/bundle/nsis
```

Kéo-thả file vào cửa sổ (hoặc bấm "Chọn file…"), chọn định dạng đích, bấm **Chuyển đổi**.
Chọn nhiều file khác định dạng thì chỉ hiện các định dạng mà tất cả đều chuyển được; file nào đã đúng định dạng đích sẽ được bỏ qua.

## Ngôn ngữ (English / Tiếng Việt)

- Chọn ở góc trên bên phải app. Lần đầu mở, app theo ngôn ngữ của Windows; lựa chọn được nhớ cho lần sau (cả cửa sổ chuyển nhanh).
- Tiêu đề menu chuột phải đổi theo: "Convert to" / "Chuyển sang". `--register` (dùng bởi bộ cài) lấy theo ngôn ngữ Windows.
- Thông báo lỗi kỹ thuật từ công cụ ngoài (Pandoc, LibreOffice, trình duyệt…) giữ nguyên tiếng Anh như công cụ báo, để dễ tra cứu.
- Thêm ngôn ngữ: tạo `app/src/i18n/<mã>.ts` theo mẫu `vi.ts` (TypeScript báo lỗi nếu thiếu khóa nào), rồi thêm vào `LANGUAGES` trong `app/src/i18n/index.tsx`.

## Menu chuột phải trong Explorer

Bật/tắt bằng ô **Menu chuột phải** ở góc trên app, hoặc bằng dòng lệnh:

```powershell
target\release\convertapp.exe --register     # thêm "Convert to ▸" vào menu chuột phải
target\release\convertapp.exe --unregister   # gỡ
```

- Ghi vào `HKCU\Software\Classes` (không cần quyền admin, chỉ áp dụng cho user hiện tại).
- Trên Windows 11, menu nằm trong **Show more options** (Shift + F10). Đưa lên menu chính là Phase 10.
- Chọn nhiều file → Explorer mở mỗi file một tiến trình; app gom lại (đợi 300 ms yên lặng) thành **một** cửa sổ nhỏ, chuyển xong thì tự đóng (bỏ tick "Tự đóng khi xong" nếu muốn giữ lại).
- Menu trỏ tới đúng file exe đã đăng ký. Nên đăng ký bằng bản **release** (`cargo tauri build --no-bundle`): bản debug mở kèm cửa sổ console, còn exe của `cargo tauri dev` cần dev server đang chạy mới hiện giao diện.

Tham số dòng lệnh của app:

```text
convertapp.exe                         mở cửa sổ chính
convertapp.exe --to jpg a.png b.png    chuyển nhanh trong cửa sổ nhỏ
convertapp.exe --register | --unregister
```

## Build & test

Yêu cầu: Rust stable, MSVC Build Tools (Windows), Node.js. Xem Phase 0 trong PLAN.md.

```powershell
cargo build                          # build mọi crate
cargo test --workspace               # chạy toàn bộ test
cargo clippy --workspace --all-targets
cargo fmt --all
npm --prefix app run typecheck       # kiểm tra TypeScript
```

## Dùng CLI

```powershell
# Xem file chuyển được sang định dạng nào
cargo run -p convertapp-cli -- formats photo.png

# Chuyển nhiều file, lưu vào thư mục out/
cargo run -p convertapp-cli -- convert --to jpg --quality 85 --out out a.png b.tiff

# Ảnh → WebP chất lượng 80, tối đa 1920px
cargo run -p convertapp-cli -- convert --to webp --quality 80 --max-width 1920 --max-height 1920 *.jpg

# Gộp ảnh và PDF thành 1 PDF (theo đúng thứ tự)
cargo run -p convertapp-cli -- convert --to pdf --merge bia.jpg noidung.pdf phuluc.pdf

# Nối ảnh chụp màn hình theo chiều dọc thành 1 ảnh
cargo run -p convertapp-cli -- convert --to png --merge shot1.png shot2.png shot3.png

# Markdown → HTML giao diện tối
cargo run -p convertapp-cli -- convert --to html --theme dark README.md

# Log chi tiết
cargo run -p convertapp-cli -- -vv convert --to ico logo.png
```

Mã thoát: `0` thành công, `1` có file lỗi (các file khác vẫn được chuyển), `2` sai tham số.

## Định dạng hỗ trợ

| Đầu vào | Đầu ra |
|---|---|
| png, jpg, webp, bmp, gif, tiff, ico | qua lại giữa các định dạng này, và pdf |
| svg | png, jpg, webp, bmp, gif, tiff, ico |
| nhiều ảnh và/hoặc PDF | **gộp** thành 1 file pdf, đúng thứ tự (mỗi ảnh 1 trang, mỗi PDF giữ mọi trang) — `--merge` |
| nhiều ảnh | **nối** thành 1 ảnh png/jpg/webp/bmp/tiff — `--merge --direction vertical\|horizontal --gap <px> [--keep-sizes]` |
| md | html (trang hoàn chỉnh, có CSS sáng/tối, `--theme auto\|light\|dark\|none`) |
| html, htm | md |
| html, md | pdf, png (chụp toàn trang) ³ |
| md, html | docx ¹ |
| docx | md (ảnh tách ra `<tên>_files/`), html (1 file, ảnh nhúng) ¹ |
| md | pdf ¹ ² |

| docx, doc, odt, rtf | pdf, docx, odt, rtf, txt ⁴ |
| xlsx, xls, ods, csv | pdf, xlsx, ods, csv ⁴ |
| pptx, ppt, odp | pdf, pptx, odp ⁴ |

¹ Cần [Pandoc](https://pandoc.org): `winget install JohnMacFarlane.Pandoc`.
² Cần thêm [Typst](https://typst.app): `winget install Typst.Typst`. Chỉ dùng khi máy không có Edge/Chrome (xem ³).
⁴ Cần **Microsoft Office** (ưu tiên, giống hệt "Save As") hoặc [LibreOffice](https://www.libreoffice.org) (`winget install TheDocumentFoundation.LibreOffice`, miễn phí). Chọn bằng `--office auto|microsoft|libreoffice` hoặc ô "Bộ Office" trong app. Xuất CSV lấy sheet đầu tiên. LibreOffice ở chỗ lạ: `CONVERTAPP_SOFFICE`.
³ Dùng Microsoft Edge (có sẵn trên Windows) hoặc Google Chrome chạy ẩn. Tùy chọn: `--paper a4|a3|a5|letter|legal`, `--landscape`, `--margin <mm>`, `--no-background`, `--width <px>` (cho PNG). Trình duyệt ở chỗ lạ: đặt `CONVERTAPP_BROWSER`.
Thiếu tool thì app hiện định dạng đó với biểu tượng 🔒 kèm lệnh cài; `convertapp-cli formats <file>` cũng in ra lý do.
Nếu tool cài ở chỗ lạ, chỉ đường dẫn bằng biến môi trường `CONVERTAPP_PANDOC` / `CONVERTAPP_TYPST`.
Dùng `--reference-doc mau.docx` (hoặc nút "File Word mẫu" trong app) để file Word theo font/kiểu tiêu đề/lề của file mẫu.

Tùy chọn ảnh (CLI / app):
- `--quality 1-100`: JPEG, WebP lossy và ảnh trong PDF. `--lossless`: WebP không mất dữ liệu.
- `--max-width`, `--max-height`: thu nhỏ cho vừa (không phóng to ảnh raster; SVG thì render đúng kích thước yêu cầu). `--scale 50`: theo %.
- `--page a4|image`: khổ trang PDF. A4 tự xoay ngang với ảnh rộng.

Ghi chú:
- Ảnh chụp điện thoại được xoay đúng chiều theo EXIF; metadata (EXIF, GPS…) không được copy sang file mới.
- Markdown hỗ trợ cú pháp GitHub: bảng, checklist, gạch ngang, chú thích, alert `> [!NOTE]`. HTML viết sẵn trong file md được giữ nguyên.
- HTML → md bỏ `<head>`, `<script>`, `<style>`; tự nhận encoding (UTF-8, `<meta charset>`, Windows-1252…).
- Đường dẫn ảnh/link tương đối giữ nguyên, nên hãy lưu file kết quả cạnh file gốc (mặc định) để ảnh vẫn hiển thị.
- Ảnh trong suốt khi chuyển sang jpg/bmp sẽ được điền nền trắng.
- Chuyển sang ico tự thu nhỏ về tối đa 256×256 (giữ tỉ lệ).
- gif động chỉ lấy frame đầu.
- Định dạng file được nhận diện theo nội dung (magic bytes), nên file bị đặt sai đuôi vẫn chuyển được.
- Không bao giờ ghi đè file có sẵn: trùng tên thì thêm `(1)`, `(2)`…

## Thêm một converter mới

1. Tạo module trong `crates/core/src/converters/`, implement trait `Converter`.
2. Thêm định dạng mới (nếu có) vào enum `Format` trong `format.rs`.
3. Đăng ký trong `Registry::with_defaults()`.
4. Viết test trong `crates/core/tests/`.
