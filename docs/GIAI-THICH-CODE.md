# Giải thích code ConvertApp

Tài liệu này dành cho người **chưa quen Rust** muốn đọc và sửa được code của ConvertApp.
Phần 1–3 kể app chạy thế nào và file nào làm gì. Phần 4 giải thích các cú pháp Rust có trong dự án,
mỗi cú pháp kèm một ví dụ thật lấy từ code. Phần 5 hướng dẫn thêm một converter mới.

> Mẹo: mở song song file này và file code đang đọc. Gặp cú pháp lạ thì tra **Phần 4**, hoặc bảng tra nhanh ở cuối.

---

## 1. Bức tranh tổng thể

Dự án là một **Cargo workspace**: một kho chứa nhiều gói Rust (gọi là *crate*), khai báo trong [Cargo.toml](../Cargo.toml) ở thư mục gốc.

```text
                ┌──────────────────────────┐
                │  app/src  (React + TS)   │  giao diện: nút, danh sách file…
                └────────────┬─────────────┘
                             │ invoke("convert_files", …)   ← gọi qua IPC của Tauri
                ┌────────────▼─────────────┐
                │  app/src-tauri  (Rust)   │  "backend" của app desktop
                └───┬──────────────────┬───┘
                    │                  │
     ┌──────────────▼───┐      ┌───────▼────────────┐
     │  crates/core     │      │  crates/shell      │  menu chuột phải (registry Windows)
     │  ENGINE chuyển   │◄─────┤                    │
     │  đổi (lõi)       │      └────────────────────┘
     └──────────────▲───┘
                    │
     ┌──────────────┴───┐
     │  crates/cli      │  chạy engine từ dòng lệnh (dùng để test nhanh)
     └──────────────────┘
```

Nguyên tắc quan trọng nhất: **mọi logic chuyển đổi nằm trong `crates/core`**.
Ba "vỏ" bên ngoài (app desktop, CLI, menu chuột phải) chỉ nhận yêu cầu từ người dùng rồi gọi vào core.
Vì vậy muốn hiểu app chuyển file thế nào, chỉ cần đọc `crates/core`.

| Crate | Tên gói | Loại | Vai trò |
|---|---|---|---|
| `crates/core` | `convertapp-core` | thư viện (`lib.rs`) | Định dạng, converter, chọn converter, ghi file an toàn |
| `crates/cli` | `convertapp-cli` | chương trình (`main.rs`) | `convertapp-cli convert --to jpg a.png` |
| `crates/shell` | `convertapp-shell` | thư viện | Ghi/xóa menu "Convert to" vào registry Windows |
| `app/src-tauri` | `convertapp` | chương trình | App desktop: mở cửa sổ, nhận lệnh từ giao diện |

> **Thư viện hay chương trình?** Crate có `src/lib.rs` là *thư viện*: crate khác dùng lại được, nhưng tự nó không chạy được.
> Crate có `src/main.rs` là *chương trình*: hàm `fn main()` là điểm bắt đầu chạy.

---

## 2. Một lần chuyển đổi đi qua những đâu?

Ví dụ: người dùng kéo `anh.png` vào app, chọn **JPG**, bấm **Chuyển đổi**.

| Bước | Ở đâu | Chuyện gì xảy ra |
|---|---|---|
| 1 | [app/src/api.ts](../app/src/api.ts) | TypeScript gọi `invoke("convert_files", { paths, target: "jpg", settings, onEvent })`. |
| 2 | [commands.rs](../app/src-tauri/src/commands.rs) `convert_files` | Tauri gọi hàm Rust cùng tên. Chuỗi `"jpg"` được đổi thành `Format::Jpeg`. Việc chuyển chạy ở luồng nền (`spawn_blocking`) để giao diện không bị đơ. |
| 3 | `commands.rs` `run_batch` | Lặp qua từng file và tạo một `Job` (file nào, sang định dạng gì, tùy chọn gì). Sau mỗi file, gửi sự kiện tiến độ về giao diện qua `Channel`. |
| 4 | [pipeline.rs](../crates/core/src/pipeline.rs) `convert` | Hàm trung tâm của engine, đọc kỹ hàm này trước tiên. |
| 4a | `Format::detect` ([format.rs](../crates/core/src/format.rs)) | Đoán định dạng thật bằng cách đọc vài byte đầu file ("magic bytes"), không chỉ nhìn đuôi file. |
| 4b | `registry.find_for` ([registry.rs](../crates/core/src/registry.rs)) | Tìm converter **đầu tiên** làm được PNG → JPG và đang dùng được trên máy. |
| 4c | `write_output` (pipeline.rs) | Tạo file tạm ẩn `.convertapp-xxxx.jpg` ngay trong thư mục đích. |
| 4d | `converter.convert(...)` | Converter (ở đây là `ImageConverter`) đọc file PNG và ghi ra file tạm. |
| 4e | `persist_noclobber` | Đổi tên file tạm thành `anh.jpg`, hoặc `anh (1).jpg` nếu tên đã có. **Không bao giờ ghi đè** file có sẵn. Nếu lỗi giữa chừng, file tạm tự bị xóa, nên không bao giờ còn sót file hỏng. |
| 5 | `commands.rs` | Báo kết quả (thành công hay lỗi) về giao diện. |

CLI ([crates/cli/src/main.rs](../crates/cli/src/main.rs)) đi đúng con đường này. Chỉ khác là bước 1–3 được thay bằng việc đọc tham số dòng lệnh.

### Converter được chọn thế nào?

[registry.rs](../crates/core/src/registry.rs) `Registry::with_defaults()` đăng ký các converter **theo thứ tự ưu tiên**:

```text
ImageConverter → SvgConverter → MarkdownConverter → HtmlToMarkdownConverter   (thuần Rust, luôn dùng được)
→ WebConverter (Chrome/Edge) → PandocConverter → MarkdownToPdfConverter        (cần chương trình ngoài)
→ MsOfficeConverter ×3 (chỉ Windows) → LibreOfficeConverter
```

Với mỗi cặp định dạng (ví dụ DOCX → PDF), `find_for` đi từ đầu danh sách xuống. Converter đầu tiên vừa **hỗ trợ cặp này** vừa **dùng được ngay** (`availability()` trả `Available`) sẽ được chọn.
Nếu không có converter nào dùng được, định dạng đó hiện trên giao diện với biểu tượng 🔒, kèm lời nhắc cần cài gì.

---

## 3. Bản đồ code

### `crates/core/src/` — engine

| File / thư mục | Nội dung |
|---|---|
| [lib.rs](../crates/core/src/lib.rs) | "Mục lục" của thư viện: khai báo các module (`mod …`) và chọn thứ gì được công khai ra ngoài (`pub use …`). |
| [format.rs](../crates/core/src/format.rs) | `enum Format`: mọi định dạng app biết (Png, Jpeg, Docx…), đuôi file, tên hiển thị, nhận diện định dạng. |
| [converter.rs](../crates/core/src/converter.rs) | `trait Converter`, "hợp đồng" mà mọi bộ chuyển đổi phải tuân theo. Cũng chứa `Availability` (dùng được không) và `Requirement` (cần cài gì). |
| [registry.rs](../crates/core/src/registry.rs) | `Registry`: danh sách converter. Trả lời các câu hỏi "file này chuyển sang được gì?" và "ai chuyển?". |
| [pipeline.rs](../crates/core/src/pipeline.rs) | `convert()`: chạy trọn một lần chuyển (xem Phần 2). |
| [job.rs](../crates/core/src/job.rs) | `Job`: một yêu cầu "file X → định dạng Y". |
| [options.rs](../crates/core/src/options.rs) | `ConvertOptions`: tất cả tùy chọn (chất lượng ảnh, khổ giấy, theme HTML…). |
| [output.rs](../crates/core/src/output.rs) | Chọn tên file kết quả không trùng (`anh (1).jpg`). |
| [error.rs](../crates/core/src/error.rs) | `enum ConvertError`: mọi loại lỗi có thể xảy ra. |
| [progress.rs](../crates/core/src/progress.rs) | `trait Progress`: báo tiến độ 0.0 → 1.0. |
| [tools.rs](../crates/core/src/tools.rs) | Tìm và chạy chương trình ngoài (Pandoc, Typst, LibreOffice), kèm lệnh cài gợi ý **theo hệ điều hành**. |
| [text.rs](../crates/core/src/text.rs) | Đọc file văn bản, tự nhận diện encoding. |
| `converters/` | Từng bộ chuyển đổi, xem bảng dưới. |
| `merge/` | Gộp nhiều file thành một: nhiều ảnh/PDF → 1 PDF (`pdf.rs`), nối ảnh (`stitch.rs`). |
| `browser/` | Điều khiển Chrome/Edge chạy ẩn để in HTML ra PDF hoặc PNG: `locate.rs` tìm trình duyệt, `session.rs` khởi động trình duyệt, `cdp.rs` nói chuyện với nó qua giao thức DevTools. |

Các converter trong `converters/`:

| Converter | File | Chuyển gì | Cần cài thêm |
|---|---|---|---|
| `ImageConverter` | `image/` (decode → transform → encode, pdf) | ảnh ↔ ảnh, ảnh → PDF | không |
| `SvgConverter` | `svg.rs` | SVG → ảnh | không |
| `MarkdownConverter` | `markdown.rs` | MD → HTML | không |
| `HtmlToMarkdownConverter` | `html.rs` | HTML → MD | không |
| `WebConverter` | `web.rs` | HTML/MD → PDF/PNG | Chrome/Edge |
| `PandocConverter`, `MarkdownToPdfConverter` | `pandoc.rs` | MD/HTML ↔ DOCX, MD → PDF | Pandoc (+ Typst) |
| `MsOfficeConverter` | `office/msoffice.rs` + `assets/office.ps1` | Office ↔ Office/PDF | Microsoft Office (chỉ Windows) |
| `LibreOfficeConverter` | `office/libreoffice.rs` | Office ↔ Office/PDF | LibreOffice |

### `app/src-tauri/src/` — backend app desktop

| File | Nội dung |
|---|---|
| [main.rs](../app/src-tauri/src/main.rs) | Điểm bắt đầu. Đọc tham số, xử lý `--register`/`--unregister`, rồi khởi động Tauri và đăng ký các plugin và command. |
| [args.rs](../app/src-tauri/src/args.rs) | Phân tích tham số dòng lệnh của app (`--to jpg a.png`). |
| [commands.rs](../app/src-tauri/src/commands.rs) | Các hàm giao diện gọi được: `inspect_files` (xem file chuyển được sang gì), `convert_files`. |
| [dto.rs](../app/src-tauri/src/dto.rs) | Kiểu dữ liệu gửi qua lại với TypeScript, phải khớp với [app/src/types.ts](../app/src/types.ts). |
| [quick.rs](../app/src-tauri/src/quick.rs) | Chế độ "chuyển nhanh" từ menu chuột phải. Explorer mở mỗi file một tiến trình; file này gom chúng lại thành một lô. |
| [window.rs](../app/src-tauri/src/window.rs) | Mở cửa sổ chính và cửa sổ chuyển nhanh. |
| [menu.rs](../app/src-tauri/src/menu.rs) | Bật/tắt menu chuột phải (gọi `crates/shell`). |

---

## 4. Rust cần biết để đọc code này

Mỗi mục: **ý nghĩa** → **ví dụ trong dự án** → so sánh với TypeScript/JavaScript nếu giúp dễ hiểu hơn.

### 4.1 Module, `pub`, `use`

```rust
// crates/core/src/lib.rs
mod converter;          // nạp file converter.rs như một module (riêng tư)
pub mod tools;          // nạp tools.rs và công khai cả module
pub use converter::{Availability, Converter, Request, Requirement};  // đưa lên "mặt tiền"
```

- `mod x;` = "có file `x.rs` (hoặc `x/mod.rs`), nạp vào đây". Giống `import` nhưng đi kèm khai báo cấu trúc.
- Mặc định mọi thứ là **riêng tư**. `pub` = công khai. `pub(crate)` = chỉ dùng được trong crate này.
- `use a::b::C;` = viết tắt, để gõ `C` thay vì `a::b::C`. `crate::` = gốc của crate hiện tại, `super::` = module cha.
- Nhờ `pub use` trong `lib.rs`, crate khác chỉ cần viết `use convertapp_core::Converter;`. Lưu ý tên gói có dấu `-`, nhưng trong code viết bằng `_`.

### 4.2 `struct`: gom dữ liệu

```rust
// crates/core/src/job.rs
pub struct Job {
    pub input: PathBuf,              // đường dẫn file
    pub target: Format,              // định dạng đích
    pub output_dir: Option<PathBuf>, // có thể không có (xem 4.5)
    pub options: ConvertOptions,
}
```

Giống `interface Job { input: string; target: Format; ... }` trong TS, nhưng là kiểu thật lúc chạy.
`struct ImageConverter;` (không có trường nào) là "struct rỗng": chỉ dùng làm cái tên để gắn hàm vào.

### 4.3 `impl`: gắn hàm vào kiểu

```rust
impl Job {
    pub fn new(input: impl Into<PathBuf>, target: Format) -> Self { ... }  // hàm tạo, gọi: Job::new(...)
    pub fn with_output_dir(mut self, dir: impl Into<PathBuf>) -> Self { ... } // gọi: job.with_output_dir(...)
}
```

- `Self` = chính kiểu đang viết (ở đây là `Job`).
- Hàm **không** có `self` là hàm tĩnh, gọi bằng `Job::new(...)`. Hàm có `self`/`&self` là phương thức, gọi bằng `job.xxx()`.
  - `&self`: chỉ đọc. `&mut self`: được sửa. `self`: "nuốt" luôn giá trị (dùng trong kiểu builder, xem 4.14).
- Kiểu trả về đứng sau `->`. Dòng cuối của hàm **không có dấu `;`** chính là giá trị trả về; không cần viết `return`.

### 4.4 `enum`: "một trong các khả năng", và `match`

Enum của Rust mạnh hơn enum của TS: mỗi biến thể **mang được dữ liệu riêng**.

```rust
// crates/core/src/converter.rs
pub enum Availability {
    Available,                                                  // không kèm dữ liệu
    Unavailable { reason: String, missing: Vec<Requirement> },  // kèm lý do + danh sách cần cài
}
```

Đọc enum bằng `match`. Trình biên dịch **bắt buộc** xử lý đủ mọi trường hợp:

```rust
// crates/core/src/registry.rs (find_for)
match converter.availability() {
    Availability::Available => return Ok(converter),
    Availability::Unavailable { reason, .. } => { /* lấy `reason`, bỏ qua các trường còn lại (..) */ }
}
```

- `_ => ...` trong `match` = "mọi trường hợp còn lại".
- `#[non_exhaustive]` trên `Format`/`ConvertError` = "sau này sẽ thêm biến thể". Code ở crate khác khi `match` phải luôn có nhánh `_`.
- `if let Some(x) = ... { }` = `match` rút gọn khi chỉ quan tâm một trường hợp.
  Edition 2024 cho phép nối nhiều điều kiện bằng `&&`, ví dụ trong `format.rs`:
  `if let (Some(content), Some(ext)) = (by_content, by_extension) && content.is_office() && ext.is_office() { ... }`

### 4.5 `Option` và `Result`: Rust không có `null` hay `throw`

| Kiểu | Nghĩa | Hai khả năng |
|---|---|---|
| `Option<T>` | có thể không có giá trị | `Some(giá_trị)` hoặc `None` |
| `Result<T, E>` | có thể lỗi | `Ok(giá_trị)` hoặc `Err(lỗi)` |

Trong dự án, `Result<T>` là viết tắt của `Result<T, ConvertError>` (khai báo ở `error.rs`).

**Dấu `?`** là thứ gặp nhiều nhất: "nếu là lỗi thì `return` lỗi đó ngay; nếu ổn thì lấy giá trị bên trong".

```rust
// crates/core/src/pipeline.rs
let input_format = detect(input)?;                                  // lỗi → hàm convert trả lỗi luôn
let converter = registry.find_for(input_format, job.target, &job.options)?;
```

Tương đương TS: `const inputFormat = detect(input); if (inputFormat is error) return inputFormat;`

Các hàm hay đi kèm:

| Viết | Nghĩa |
|---|---|
| `x.unwrap()` | lấy giá trị, **crash** nếu là `None`/`Err`. Chỉ dùng trong test hoặc khi chắc chắn. |
| `x.expect("lý do")` | như `unwrap` nhưng kèm thông báo khi crash. |
| `x.unwrap_or(mặc_định)` | `None`/`Err` thì dùng giá trị mặc định. |
| `x.map(f)` | biến đổi giá trị bên trong nếu có. |
| `x.ok_or_else(\|\| lỗi)` | đổi `Option` thành `Result` (không có giá trị → lỗi). |
| `x.map_err(f)` | đổi kiểu lỗi, ví dụ lỗi I/O → `ConvertError::Io`. |
| `x.and_then(f)` | nối tiếp một bước nữa cũng trả `Option`/`Result`. |
| `x.is_some_and(\|v\| ...)` | có giá trị **và** giá trị thỏa điều kiện. |
| `cond.then_some(v)` | `true` → `Some(v)`, `false` → `None`. |

### 4.6 `trait`: "hợp đồng" (giống `interface`)

```rust
// crates/core/src/converter.rs
pub trait Converter: Send + Sync + fmt::Debug {
    fn id(&self) -> &'static str;                     // bắt buộc viết
    fn inputs(&self) -> &[Format];                    // bắt buộc viết
    fn outputs(&self, input: Format) -> Vec<Format>;  // bắt buộc viết
    fn availability(&self) -> Availability { Availability::Available } // có sẵn mặc định, viết lại nếu cần
    fn convert(&self, request: &Request<'_>, progress: &dyn Progress) -> Result<()>;
}

// crates/core/src/converters/image/mod.rs
impl Converter for ImageConverter {   // "ImageConverter thực hiện hợp đồng Converter"
    fn id(&self) -> &'static str { "image" }
    ...
}
```

- `: Send + Sync + Debug` = ai thực hiện `Converter` cũng phải dùng được an toàn giữa nhiều luồng và in ra được để debug.
- **`dyn Converter`** = "một thứ nào đó thực hiện `Converter`, chưa biết cụ thể là gì".
  `Registry` lưu `Vec<Box<dyn Converter>>`: một danh sách gồm nhiều loại converter khác nhau. `Box` = để đối tượng trên vùng nhớ heap.
- **`impl Trait`** ở tham số, ví dụ `input: impl Into<PathBuf>`: "nhận bất kỳ kiểu nào đổi được thành `PathBuf`".
  Nhờ vậy `Job::new("a.png", ...)` nhận được cả `&str`, `String` lẫn `PathBuf`.
- Mẹo trong `progress.rs`: `impl<F> Progress for F where F: Fn(f32)` = **mọi hàm/closure** nhận `f32` đều tự động là một `Progress`,
  nên có thể truyền thẳng `&|x| println!("{x}")`.

### 4.7 Mượn (`&`) và vòng đời (`'a`, `'static`)

Rust không có garbage collector. Mỗi giá trị có **một chủ sở hữu**; khi chủ hết phạm vi, giá trị bị giải phóng.
Muốn cho hàm khác dùng mà không trao quyền sở hữu thì **cho mượn**:

| Viết | Nghĩa |
|---|---|
| `x: Job` | nhận luôn quyền sở hữu; người gọi không dùng `x` được nữa. |
| `x: &Job` | mượn để đọc. |
| `x: &mut Job` | mượn để sửa (tại một thời điểm chỉ một chỗ được mượn kiểu này). |

Hai cặp kiểu hay gặp: `String` (chuỗi sở hữu, sửa được) và `&str` (mượn một đoạn chuỗi); `PathBuf` (sở hữu) và `&Path` (mượn).

**Vòng đời** (`'a`, `'static`) chỉ là nhãn nói "tham chiếu này sống được bao lâu". Trình biên dịch dùng nhãn này để chặn lỗi trỏ vào bộ nhớ đã giải phóng.

```rust
// crates/core/src/converter.rs
pub struct Request<'a> {
    pub input: &'a Path,              // Request chỉ mượn đường dẫn, sống không lâu hơn nó
    pub options: &'a ConvertOptions,
    ...
}
```

- `&'static str` = chuỗi viết cứng trong code, sống suốt chương trình. Ví dụ: `"Pandoc"`, `"brew install pandoc"`.
- `'_` = "có vòng đời, để trình biên dịch tự suy ra".
- Khi đọc code, có thể **bỏ qua các nhãn `'a`**: chúng không làm thay đổi logic.

`.clone()` = sao chép thật một giá trị. `Arc::clone(&x)` = tạo thêm một "tay cầm" trỏ vào cùng dữ liệu, không sao chép dữ liệu (xem 4.11).

### 4.8 Closure và iterator: thay cho vòng `for`

Code trong dự án hay viết theo "chuỗi" giống `array.map().filter()` của JS:

```rust
// crates/core/src/converters/image/mod.rs
encode::OUTPUTS
    .iter()                     // duyệt từng phần tử (mượn)
    .copied()                   // lấy bản sao từng Format (Format nhỏ, sao chép rẻ)
    .filter(|&f| f != input)    // bỏ định dạng trùng với đầu vào
    .collect()                  // gom kết quả lại thành Vec<Format>
```

- `|x| x + 1` là **closure** (hàm vô danh), giống `(x) => x + 1` trong JS. `|| ...` = closure không tham số.
  `move |x| ...` = closure "mang theo" (sở hữu) các biến nó dùng; cần khi chạy ở luồng khác.
- Các bước hay gặp: `map`, `filter`, `filter_map` (lọc và biến đổi cùng lúc), `flat_map`, `find` (phần tử đầu tiên thỏa điều kiện),
  `any`/`all`, `chain` (nối hai chuỗi), `enumerate` (kèm chỉ số), `collect` (gom lại).
- Chuỗi iterator **lười**: chỉ chạy khi có `collect`/`find`/`for`… ở cuối.

Ví dụ đọc từng bước, trong `tools.rs` `Tool::locate`:

```rust
path_dirs.into_iter()          // các thư mục trong PATH
    .chain(known_dirs)         // + thư mục quen thuộc (theo biến môi trường)
    .chain(system_dirs)        // + thư mục hệ thống (/opt/homebrew/bin…)
    .map(|dir| dir.join(&file_name))         // ghép thành đường dẫn đầy đủ tới file chương trình
    .find(|candidate| candidate.is_file())   // lấy cái đầu tiên có thật
    .or_else(|| self.app_path.and_then(registered_app_path))  // không thấy thì hỏi registry Windows
```

### 4.9 Code theo hệ điều hành: `#[cfg(...)]` và `cfg!(...)`

Có hai cách, và chúng khác nhau quan trọng:

| Cách | Ví dụ | Tác dụng |
|---|---|---|
| `#[cfg(windows)]` (thuộc tính) | trên `fn registered_app_path` trong `tools.rs` | Đoạn code bị **bỏ hẳn** khi build cho hệ khác. Dùng khi code gọi thư viện chỉ có trên Windows (`winreg`). |
| `cfg!(windows)` (macro, trả `true`/`false`) | `if cfg!(windows) { "winget …" } else { … }` | Code vẫn được biên dịch trên mọi hệ, chỉ là điều kiện cố định. Dùng khi code chạy được ở đâu cũng được. |

Hay gặp: `windows`, `unix`, `target_os = "macos"`, `target_os = "linux"`, `not(...)`, `all(...)`, `any(...)`.
Ví dụ `#[cfg(all(unix, not(target_os = "macos")))]` = "Linux và các Unix khác, trừ macOS".

`#[cfg(test)]` = chỉ biên dịch khi chạy `cargo test`. Các khối `mod tests { ... }` ở cuối file là test của file đó.

Trong `Cargo.toml`, `[target."cfg(windows)".dependencies]` = thư viện chỉ tải khi build cho Windows (ví dụ `winreg`).

### 4.10 `#[derive(...)]` và các thuộc tính khác

`#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]` = nhờ trình biên dịch **tự viết** các khả năng thông dụng:

| Derive | Cho phép |
|---|---|
| `Debug` | in ra để debug: `{:?}` |
| `Clone` | `.clone()` |
| `Copy` | gán là tự sao chép (chỉ cho kiểu nhỏ như `Format`) |
| `PartialEq`, `Eq` | so sánh `==` |
| `Hash` | làm khóa trong HashMap/HashSet |
| `PartialOrd`, `Ord` | so sánh `<`, sắp xếp |
| `Default` | tạo giá trị mặc định: `ConvertOptions::default()` |
| `Serialize`, `Deserialize` (serde) | chuyển sang/từ JSON để gửi cho TypeScript (trong `dto.rs`) |
| `thiserror::Error` | biến enum thành kiểu lỗi chuẩn, với `#[error("…")]` là câu thông báo |

Các thuộc tính khác:
- `#[must_use]`: cảnh báo nếu gọi hàm mà bỏ quên kết quả.
- `#[serde(rename_all = "camelCase")]`: đổi `install_command` (Rust) thành `installCommand` (JSON/TS).
- `#[tauri::command]`: cho phép TypeScript gọi hàm này bằng `invoke("tên_hàm")`.

### 4.11 Nhiều luồng: `Arc`, `Mutex`, `OnceLock`, `async`

| Thứ | Dùng để | Ví dụ |
|---|---|---|
| `Arc<T>` | chia sẻ một giá trị giữa nhiều luồng; tự giải phóng khi không ai dùng nữa | `AppState { registry: Arc<Registry> }` trong `commands.rs` |
| `Mutex<T>` | khóa: mỗi lúc chỉ một luồng được chạm vào dữ liệu | `QuickQueue` trong `quick.rs`, `OFFICE_LOCK` trong `msoffice.rs` |
| `OnceLock<T>` | tính một lần, nhớ mãi | dò trình quản lý gói Linux trong `tools.rs` |
| `async fn` + `.await` | hàm bất đồng bộ (giống JS) | các `#[tauri::command] pub async fn …` |
| `spawn_blocking(move \|\| ...)` | chạy việc nặng (đọc/ghi file) ở luồng riêng để giao diện không đơ | `convert_files` |

Engine (`crates/core`) hoàn toàn **đồng bộ** (không có `async`). Chỉ lớp Tauri dùng `async`.

### 4.12 Hằng số: `const` và `static`

```rust
pub const PANDOC: Tool = Tool { name: "Pandoc", program: "pandoc", ... };   // tools.rs
const MACOS: bool = cfg!(target_os = "macos");
static OFFICE_LOCK: Mutex<()> = Mutex::new(());                              // msoffice.rs
```

`const` = giá trị được chép vào mọi chỗ dùng. `static` = đúng **một** vùng nhớ dùng chung (cần cho khóa).
Trong `const` vẫn dùng được `if cfg!(...) { A } else { B }` để chọn giá trị theo hệ điều hành; `PANDOC`, `LIBREOFFICE`, `TYPST` đều làm vậy.

### 4.13 Macro: những thứ có dấu `!`

`println!`, `format!`, `vec!`, `assert!`, `assert_eq!`, `panic!`, `cfg!`, `include_str!`. Chúng là macro (sinh code lúc biên dịch), không phải hàm.

- `format!("{} ({n}).{ext}", stem)` = tạo chuỗi; giống template string của JS. `{x}` lấy biến `x`, `{:?}` in kiểu debug.
- `vec![a, b]` = tạo `Vec` (mảng động).
- `include_str!("../../../assets/office.ps1")` = nhúng nội dung file vào chương trình lúc biên dịch.
- `tracing::debug!(...)`, `tracing::info!(...)` = ghi log (xem bằng `convertapp-cli -vv`).

### 4.14 Các mẫu thiết kế hay gặp trong dự án

- **Builder `with_*`**: `Job::new(file, Format::Jpeg).with_output_dir("out").with_options(opts)`. Mỗi hàm nhận `self`, sửa, rồi trả lại `self`.
- **Struct update**: `ConvertOptions { office: ..., ..ConvertOptions::default() }` = "các trường còn lại lấy giá trị mặc định".
- **Type alias**: `pub type Result<T, E = ConvertError> = std::result::Result<T, E>;` = đặt tên ngắn cho kiểu dài.
- **Không ghi đè, ghi qua file tạm**: xem `write_output` trong `pipeline.rs`.
- **Khóa mọi thứ phụ thuộc hệ điều hành vào một chỗ**: `tools.rs`, `browser/locate.rs`, `shell/src/windows.rs`.

### 4.15 Comment tài liệu

- `//!` ở đầu file = mô tả **cả file/module**. Nên đọc phần này đầu tiên, vì hầu hết file đều có đoạn tóm tắt rất rõ.
- `///` = mô tả cho hàm/kiểu ngay bên dưới. Hiện khi rê chuột trong VS Code.
- `` [`Format`] `` trong comment = link tới kiểu đó (bấm được khi xem bằng `cargo doc`).

---

## 5. Ví dụ: thêm một converter mới

Giả sử muốn thêm chuyển **TXT → HTML**:

1. **Định dạng**: `Format::Txt` đã có sẵn nên bỏ qua bước này. Với định dạng mới hoàn toàn, thêm biến thể vào `enum Format` và `Format::ALL` trong [format.rs](../crates/core/src/format.rs), rồi bổ sung `extensions()`, `display_name()`, `category()`.
   Trình biên dịch sẽ chỉ ra mọi `match` còn thiếu trường hợp mới; sửa lần lượt theo đó.
2. **Viết converter**: tạo `crates/core/src/converters/txt.rs`:

   ```rust
   use crate::{Converter, Format, Progress, Request, Result};

   /// TXT → HTML (bọc văn bản trong <pre>).
   #[derive(Debug, Default, Clone, Copy)]
   pub struct TxtConverter;

   impl Converter for TxtConverter {
       fn id(&self) -> &'static str { "txt" }
       fn inputs(&self) -> &[Format] { &[Format::Txt] }
       fn outputs(&self, input: Format) -> Vec<Format> {
           if input == Format::Txt { vec![Format::Html] } else { Vec::new() }
       }
       fn convert(&self, request: &Request<'_>, _progress: &dyn Progress) -> Result<()> {
           let text = std::fs::read_to_string(request.input)
               .map_err(|e| crate::ConvertError::io(request.input, e))?;
           let html = format!("<pre>{}</pre>", text.replace('&', "&amp;").replace('<', "&lt;"));
           std::fs::write(request.output, html)
               .map_err(|e| crate::ConvertError::io(request.output, e))
       }
   }
   ```

   Không cần lo tên file, ghi đè hay file tạm: pipeline đã lo hết. Converter chỉ việc **đọc `request.input`, ghi `request.output`**.
   Nếu converter cần chương trình ngoài, viết thêm `fn availability()` dùng một `Tool` trong `tools.rs` (xem `PandocConverter`).
3. **Khai báo module**: thêm `mod txt;` và `pub use txt::TxtConverter;` vào [converters/mod.rs](../crates/core/src/converters/mod.rs).
4. **Đăng ký**: thêm `.register(TxtConverter)` trong `Registry::with_defaults()` ([registry.rs](../crates/core/src/registry.rs)). Vị trí trong danh sách quyết định độ ưu tiên.
5. **Test**: viết test trong `crates/core/tests/`, rồi chạy `cargo test --workspace`.

Giao diện, CLI và menu chuột phải **tự thấy** định dạng mới, vì cả ba đều hỏi `Registry`.

---

## 6. Mẹo đọc code

- **Thứ tự nên đọc**: `lib.rs` → `format.rs` → `converter.rs` → `pipeline.rs` → `registry.rs` → `converters/image/mod.rs` (converter đơn giản nhất) → `commands.rs`.
- **VS Code + rust-analyzer** (đã gợi ý trong `.vscode/extensions.json`): rê chuột để xem kiểu và tài liệu, F12 để nhảy tới định nghĩa, Shift+F12 để xem nơi dùng.
  rust-analyzer còn hiện **kiểu được suy ra** ngay trên dòng code, nên đọc chuỗi iterator dễ hơn nhiều.
- **Tài liệu dạng web**: chạy lệnh dưới để mở toàn bộ comment `///` thành trang web có link:

  ```bash
  cargo doc --workspace --no-deps --open
  ```

- **Xem engine làm gì**: chạy với log chi tiết, hiện converter nào được chọn và lệnh ngoài nào được gọi:

  ```bash
  cargo run -p convertapp-cli -- -vv convert --to jpg anh.png
  ```

- **Đọc lỗi biên dịch**: thông báo lỗi của Rust thường **chỉ luôn cách sửa** (`help: ...`). Hãy đọc hết thông báo.
- Gặp cú pháp lạ mà không có ở đây: tra [Rust Book tiếng Anh](https://doc.rust-lang.org/book/) hoặc [Rust by Example](https://doc.rust-lang.org/rust-by-example/).

---

## Bảng tra nhanh ký hiệu

| Ký hiệu | Nghĩa |
|---|---|
| `fn f(x: T) -> U` | hàm nhận `T`, trả `U` |
| `let x = …` / `let mut x = …` | biến không đổi / biến sửa được |
| `&x`, `&mut x` | mượn để đọc / để sửa |
| `x?` | lỗi thì trả lỗi ngay, ổn thì lấy giá trị |
| `Some(x)` / `None` | có / không có giá trị |
| `Ok(x)` / `Err(e)` | thành công / lỗi |
| `A::b` | hàm/hằng/biến thể `b` thuộc `A` (`Format::Png`, `Job::new`) |
| `x.f()` | gọi phương thức |
| `\|a\| …` | closure (hàm vô danh) |
| `'a`, `'static` | nhãn vòng đời, có thể bỏ qua khi đọc logic |
| `impl X for Y` | kiểu `Y` thực hiện trait `X` |
| `dyn X` | "thứ gì đó thực hiện `X`" |
| `Box<T>`, `Arc<T>`, `Vec<T>` | hộp trên heap / chia sẻ giữa luồng / mảng động |
| `#[...]` | thuộc tính gắn lên thứ bên dưới (derive, cfg, test…) |
| `xxx!(…)` | macro |
| `..` | "phần còn lại" (trong `match` hoặc khi tạo struct) |
| `_` | "không quan tâm" |
| `::<T>` | chỉ rõ kiểu, ví dụ `"jpg".parse::<Format>()` |
