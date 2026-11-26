# Installing ConvertApp

[English](#english) · [Tiếng Việt](#tiếng-việt)

ConvertApp is free and not signed with a paid Apple / Microsoft developer certificate. Your system will warn you the first time you open it. This is expected; the steps below get past it once, and the app opens normally afterwards.

---

## English

Download from the [latest release](https://github.com/ds-kell/convert-multi-file/releases/latest).

### Windows 10 / 11

1. Download `ConvertApp_<version>_x64-setup.exe` and run it.
2. If **"Windows protected your PC"** (SmartScreen) appears: click **More info** → **Run anyway**.
3. Follow the installer. The right-click **Convert to ▸** menu is added automatically.

### macOS

1. Pick the right file:
   - **Apple Silicon** (M1, M2, M3, M4…): `ConvertApp_<version>_aarch64.dmg`
   - **Intel**: `ConvertApp_<version>_x64.dmg`

   Not sure? Apple menu  → **About This Mac**: "Chip Apple M…" means Apple Silicon, "Processor Intel…" means Intel.
2. Open the `.dmg` and drag **ConvertApp** into **Applications**.
3. Open ConvertApp. macOS shows:

   > *"ConvertApp" Not Opened — Apple could not verify "ConvertApp" is free of malware…*

   Click **Done** (do **not** click *Move to Trash*).
4. Open **System Settings → Privacy & Security**, scroll down to **Security**. Next to *"ConvertApp" was blocked…* click **Open Anyway**, enter your password / Touch ID, then click **Open**.

You only do this once. On macOS 15 (Sequoia) and later, right-click → *Open* no longer skips this step, so use System Settings as above.

**Alternative (Terminal), one command:**

```bash
xattr -cr /Applications/ConvertApp.app
```

**Seeing *"ConvertApp" is damaged and can't be opened*?** You have a build older than v0.1.1. Download the latest release, or run the `xattr` command above.

### Linux

- **Debian / Ubuntu:** download the `.deb`, then `sudo apt install ./ConvertApp_<version>_amd64.deb`
- **Any distro:** download the `.AppImage`, then `chmod +x ConvertApp_*.AppImage` and run it.

### Optional tools

Word, Excel, PowerPoint and some PDF conversions need free external programs (Pandoc, LibreOffice…). You don't need to install them up front: when a format needs one, the app shows a 🔒 and tells you exactly what to install, with a download button and a copy-able install command.

---

## Tiếng Việt

Tải về tại [bản phát hành mới nhất](https://github.com/ds-kell/convert-multi-file/releases/latest).

### Windows 10 / 11

1. Tải `ConvertApp_<version>_x64-setup.exe` và chạy.
2. Nếu hiện **"Windows protected your PC"** (SmartScreen): bấm **More info** → **Run anyway**.
3. Làm theo trình cài đặt. Menu chuột phải **Convert to ▸** được thêm tự động.

### macOS

1. Chọn đúng file:
   - **Apple Silicon** (M1, M2, M3, M4…): `ConvertApp_<version>_aarch64.dmg`
   - **Intel**: `ConvertApp_<version>_x64.dmg`

   Không biết máy loại nào? Bấm logo Apple  → **About This Mac**: thấy "Chip Apple M…" là Apple Silicon, "Processor Intel…" là Intel.
2. Mở file `.dmg`, kéo **ConvertApp** vào thư mục **Applications**.
3. Mở ConvertApp. macOS sẽ báo:

   > *"ConvertApp" Not Opened — Apple could not verify "ConvertApp" is free of malware…*

   Bấm **Done** (**đừng** bấm *Move to Trash*).
4. Mở **System Settings → Privacy & Security**, kéo xuống phần **Security**. Ở dòng *"ConvertApp" was blocked…* bấm **Open Anyway**, nhập mật khẩu máy / Touch ID, rồi bấm **Open**.

Chỉ cần làm một lần. Từ macOS 15 (Sequoia) trở đi, chuột phải → *Open* không còn bỏ qua được bước này, nên hãy làm qua System Settings như trên.

**Cách khác (Terminal), một lệnh:**

```bash
xattr -cr /Applications/ConvertApp.app
```

**Gặp lỗi *"ConvertApp" is damaged and can't be opened*?** Bạn đang dùng bản cũ hơn v0.1.1. Hãy tải bản mới nhất, hoặc chạy lệnh `xattr` ở trên.

### Linux

- **Debian / Ubuntu:** tải file `.deb`, rồi `sudo apt install ./ConvertApp_<version>_amd64.deb`
- **Mọi bản phân phối:** tải file `.AppImage`, rồi `chmod +x ConvertApp_*.AppImage` và chạy.

### Công cụ bổ sung (không bắt buộc)

Chuyển Word, Excel, PowerPoint và một số kiểu PDF cần chương trình miễn phí bên ngoài (Pandoc, LibreOffice…). Không cần cài trước: khi định dạng nào cần, app sẽ hiện 🔒 và chỉ rõ cần cài gì, kèm nút tải về và lệnh cài để sao chép.
