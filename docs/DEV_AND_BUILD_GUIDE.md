# 🛠️ Hướng Dẫn Dev & Build — Antigravity Tools

> Tài liệu này mô tả đầy đủ quy trình phát triển (development), đóng gói (build), phân phối (distribution), và cập nhật (update) cho ứng dụng **Antigravity Tools** — được xây dựng bằng **Tauri v2 + React + TypeScript**.

---

## 📋 Mục Lục

- [Môi Trường Phát Triển (Development)](#môi-trường-phát-triển-development)
- [Môi Trường Production (Build)](#môi-trường-production-build)
- [Sử Dụng File Build](#sử-dụng-file-build)
- [Cập Nhật Phiên Bản Mới](#cập-nhật-phiên-bản-mới)
- [Cấu Trúc Thư Mục Output](#cấu-trúc-thư-mục-output)
- [Debug Nâng Cao](#debug-nâng-cao)

---

## 🔧 Môi Trường Phát Triển (Development)

### Yêu Cầu Hệ Thống

| Công cụ | Phiên bản | Ghi chú |
|---|---|---|
| Node.js | ≥ 20 LTS | Dùng `nvm` hoặc `fnm` để quản lý |
| Rust | stable (latest) | Cài qua [rustup.rs](https://rustup.rs) |
| LLVM/Clang | ≥ 16 | Bắt buộc cho `boring-sys2` FFI binding |
| Tauri CLI | v2 (qua npm) | Đã có trong `devDependencies` |
| WebView2 Runtime | Mới nhất | Windows 10/11: thường đã có sẵn |

### Cài Đặt Lần Đầu

```bash
# 1. Clone repo
git clone https://github.com/lbjlaq/Antigravity-Manager.git
cd Antigravity-Manager

# 2. Cài đặt dependencies Node
npm install

# 3. Kiểm tra Rust toolchain
rustup update stable
rustup target add x86_64-pc-windows-msvc   # Windows

# 4. Đảm bảo LLVM đã cài và LIBCLANG_PATH được set
# Ví dụ Windows (thêm vào System Environment Variables):
# LIBCLANG_PATH = C:\Program Files\LLVM\bin
```

### Chạy Dev Mode

```bash
npm run tauri dev
```

**Điều xảy ra khi chạy lệnh này:**

1. Tauri CLI gọi `beforeDevCommand` → chạy `npm run dev` (Vite dev server) tại `http://localhost:1420`
2. Rust backend được biên dịch (lần đầu sẽ lâu ~5-15 phút tùy máy, các lần sau nhanh hơn nhờ cache)
3. Một cửa sổ desktop native mở ra, load frontend từ Vite dev server
4. **Hot reload** hoạt động: thay đổi TypeScript/React → tự động reload frontend; thay đổi Rust → tự động recompile backend

### Debug Mode (Log Chi Tiết)

```bash
npm run tauri:debug
# Tương đương: RUST_LOG=debug npm run tauri dev
```

> Lưu ý: Lệnh `tauri:debug` dùng cú pháp Unix (`RUST_LOG=debug`). Trên Windows PowerShell nên dùng:
> ```powershell
> $env:RUST_LOG="debug"; npm run tauri dev
> ```

---

## 🏭 Môi Trường Production (Build)

### Lệnh Build

```bash
npm run tauri build
```

**Điều xảy ra khi chạy lệnh này:**

1. `beforeBuildCommand` được gọi → chạy `tsc && vite build` → output vào thư mục `dist/`
2. Rust backend được biên dịch ở chế độ **release** (tối ưu hóa cao, strip debug symbols)
3. Tauri đóng gói toàn bộ (frontend + backend + WebView2 bootstrap) thành installer
4. Artifacts được tạo ra trong `src-tauri/target/release/bundle/`

### Thời Gian Build

| Lần chạy | Thời gian ước tính |
|---|---|
| Lần đầu tiên (cold build) | 15-30 phút |
| Các lần tiếp theo (incremental) | 3-10 phút |

### Các Target Build (Windows)

Vì `tauri.conf.json` đang cấu hình `"targets": "all"`, Tauri sẽ tạo ra **tất cả** các định dạng sau:

| File | Định dạng | Mô tả |
|---|---|---|
| `Antigravity Tools_4.x.x_x64-setup.exe` | NSIS Installer | **Dùng để phân phối cho người dùng** |
| `Antigravity Tools_4.x.x_x64_en-US.msi` | MSI Installer | Chuẩn doanh nghiệp (Group Policy) |
| `Antigravity Tools.exe` (trong `release/`) | Executable thô | Chạy trực tiếp, không cài đặt |

### Build Cho Target Cụ Thể

```bash
# Chỉ build NSIS installer (nhanh hơn)
npm run tauri build -- --bundles nsis

# Chỉ build MSI
npm run tauri build -- --bundles msi

# Build với verbose log
npm run tauri build -- --verbose
```

---

## 📦 Sử Dụng File Build

### Tìm File Sau Khi Build

```
src-tauri/target/release/bundle/
├── nsis/
│   └── Antigravity Tools_4.x.x_x64-setup.exe   ← File cài đặt chính
├── msi/
│   └── Antigravity Tools_4.x.x_x64_en-US.msi
└── updater/
    └── Antigravity Tools_4.x.x_x64-setup.exe.sig  ← Chữ ký cho auto-updater
```

### Cài Đặt Trên Máy (Windows)

1. Copy file `Antigravity Tools_4.x.x_x64-setup.exe` sang máy người dùng
2. Double-click để chạy installer
3. Windows SmartScreen có thể hiện cảnh báo → chọn **"Run anyway"** (vì chưa có code signing certificate)
4. App sẽ được cài vào:
   - **Executable**: `C:\Users\<User>\AppData\Local\Antigravity Tools\`
   - **App data**: `C:\Users\<User>\AppData\Roaming\com.lbjlaq.antigravity-tools\`
5. Sau khi cài, shortcut xuất hiện ở Start Menu và Desktop

### Chạy Không Cần Cài (Portable)

```
src-tauri/target/release/Antigravity Tools.exe
```

File này chạy trực tiếp nhưng **không có auto-updater** và không tạo shortcut. Dùng để test nhanh.

---

## 🔄 Cập Nhật Phiên Bản Mới

### Kịch Bản 1: Auto-Update (Đã Tích Hợp Sẵn) ✅

Project đã cấu hình **Tauri Updater Plugin** trong `tauri.conf.json`:

```json
"updater": {
  "active": true,
  "endpoints": [
    "https://github.com/lbjlaq/Antigravity-Manager/releases/latest/download/updater.json"
  ],
  "dialog": true
}
```

Auto-update hoạt động như sau:
1. Khi app khởi động (hoặc theo lịch), nó kiểm tra `updater.json` trên GitHub Releases
2. Nếu có phiên bản mới → hiện dialog hỏi người dùng
3. Người dùng bấm "Update" → app tự tải và cài bản mới
4. **Không cần gỡ cài đặt bản cũ** — updater tự thay thế

### Kịch Bản 2: Cài Đè Thủ Công ✅

**Với NSIS Installer (`.exe`): CÀI ĐÈ TRỰC TIẾP — Không cần gỡ bản cũ**

```
Bước 1: Chạy installer mới (setup.exe)
Bước 2: Installer tự detect bản cũ và uninstall-then-install trong nền
Bước 3: Done ✅
```

> 💡 **Quy tắc vàng**: Với Tauri NSIS installer, bạn **luôn có thể cài đè** mà không cần uninstall trước. Installer sẽ tự xử lý.

**Với MSI (`.msi`): CÀI ĐÈ ĐƯỢC nhưng cần cùng Product Code**

MSI có strict version management. Nếu gặp lỗi conflict, gỡ bản cũ trước qua:
- `Settings → Apps → Antigravity Tools → Uninstall`

### Kịch Bản 3: Gỡ Hoàn Toàn (Clean Uninstall)

```
Settings → Apps → Installed Apps → "Antigravity Tools" → Uninstall
```

Hoặc qua Control Panel → Programs. App data (accounts, settings) vẫn còn ở:
```
C:\Users\<User>\AppData\Roaming\com.lbjlaq.antigravity-tools\
```

Để xóa hoàn toàn, xóa thư mục trên sau khi uninstall.

---

## 🗂️ Cấu Trúc Thư Mục Output

```
src-tauri/target/
├── debug/                          # Build debug (dùng bởi `tauri dev`)
│   └── Antigravity Tools.exe       # Executable debug
└── release/                        # Build production (dùng bởi `tauri build`)
    ├── Antigravity Tools.exe        # Executable thô (portable)
    └── bundle/
        ├── nsis/
        │   ├── Antigravity Tools_<version>_x64-setup.exe          ← Installer chính
        │   └── Antigravity Tools_<version>_x64-setup.exe.sig      ← Chữ ký updater
        ├── msi/
        │   └── Antigravity Tools_<version>_x64_en-US.msi
        └── updater/
            └── updater.json                                        ← Manifest cho auto-update
```

> ⚠️ **Không commit thư mục `target/`** — đã có trong `.gitignore`. Size có thể lên đến 2-5GB.

---

## 🐛 Debug Nâng Cao

### Xem Log Rust Backend

```powershell
# Windows PowerShell
$env:RUST_LOG="debug"; npm run tauri dev

# Hoặc log theo module cụ thể
$env:RUST_LOG="antigravity_tools=debug,tauri=warn"; npm run tauri dev
```

### Xem DevTools Frontend

Trong dev mode, chuột phải vào cửa sổ app → **"Inspect Element"** để mở DevTools (như Chrome).

Trong **production build**, DevTools bị tắt theo mặc định. Để bật lại khi debug production:

```json
// src-tauri/tauri.conf.json (tạm thời, đừng commit lên production)
"app": {
  "windows": [{
    "devtools": true
  }]
}
```

### Kiểm Tra Version

```bash
# Xem version Tauri CLI
npx tauri --version

# Xem version Rust
rustc --version
cargo --version

# Xem version app
cat package.json | grep version   # Linux/Mac
(Get-Content package.json | ConvertFrom-Json).version   # PowerShell
```

---

## 📝 Tóm Tắt Nhanh

| Tình huống | Lệnh / Hành động |
|---|---|
| **Phát triển hàng ngày** | `npm run tauri dev` |
| **Debug chi tiết** | `$env:RUST_LOG="debug"; npm run tauri dev` |
| **Build bản release** | `npm run tauri build` |
| **File cài đặt output** | `src-tauri/target/release/bundle/nsis/*.exe` |
| **Cài trên máy người dùng** | Double-click `.exe` installer |
| **Cập nhật lên bản mới** | Cài đè trực tiếp (NSIS) HOẶC auto-update in-app |
| **Gỡ cài đặt** | Settings → Apps → Uninstall |

---

*Cập nhật lần cuối: 2026-03-29 | Tauri v2 | Antigravity Tools v4.x*
