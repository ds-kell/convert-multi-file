# ConvertApp

App desktop chuyển đổi file (ảnh, Markdown, HTML, Office), dùng được bằng chuột phải trong Explorer hoặc mở app như bình thường.
Viết bằng Rust + Tauri v2. Kế hoạch tổng thể xem [PLAN.md](PLAN.md), tiến độ xem [TASKS.md](TASKS.md).

## Cấu trúc

| Thư mục | Nội dung |
|---|---|
| `crates/core` | Engine chuyển đổi: `Format`, trait `Converter`, `Registry`, pipeline, các converter |
| `crates/cli` | `convertapp-cli`: dùng engine từ dòng lệnh, tiện để test nhanh |

## Build & test

Yêu cầu: Rust stable, MSVC Build Tools (Windows). Xem Phase 0 trong PLAN.md.

```powershell
cargo build                          # build mọi crate
cargo test --workspace               # chạy toàn bộ test
cargo clippy --workspace --all-targets
cargo fmt --all
```

## Dùng CLI

```powershell
# Xem file chuyển được sang định dạng nào
cargo run -p convertapp-cli -- formats photo.png

# Chuyển nhiều file, lưu vào thư mục out/
cargo run -p convertapp-cli -- convert --to jpg --quality 85 --out out a.png b.tiff

# Log chi tiết
cargo run -p convertapp-cli -- -vv convert --to ico logo.png
```

Mã thoát: `0` thành công, `1` có file lỗi (các file khác vẫn được chuyển), `2` sai tham số.

## Định dạng hỗ trợ

| Đầu vào | Đầu ra |
|---|---|
| png, jpg, bmp, gif, tiff, ico | qua lại giữa các định dạng này |

Ghi chú:
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
