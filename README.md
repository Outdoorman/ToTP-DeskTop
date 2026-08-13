# TOTP Desk

一个面向 Windows 的轻量离线 TOTP 桌面验证器，使用 Rust、Tauri 2 和原生 TypeScript 构建。

## 功能

- 手动添加 Base32 Seed
- 文本批量导入：Seed、`otpauth://totp`、`otpauth-migration://`
- 从摄像头、图片或剪贴板截图扫描二维码
- 导入/导出单个账号或完整 JSON 备份
- SHA-1、SHA-256、SHA-512，支持 6 位或 8 位验证码
- Windows DPAPI 用户级加密落盘；前端不持久化密钥
- Rust 侧密钥缓冲区使用 `zeroize`
- Release 启用 LTO、单 codegen unit 与 `panic=abort`

## Rust 版本

项目通过 `rust-toolchain.toml` 锁定 Rust 1.97.1。不要使用 Ubuntu `apt` 安装的旧版 Cargo；Rust 2024 Edition 至少需要 Cargo 1.85。

检查版本：

```bash
rustc --version
cargo --version
```

## 在 Windows 原生构建

需要 Node.js、WebView2、Rust MSVC 工具链，以及 Visual Studio C++ Build Tools 的“使用 C++ 的桌面开发”工作负载。

```powershell
rustup update
rustup default stable-msvc
npm ci
npm run tauri build
```

安装包位于 `src-tauri/target/release/bundle/`。

## 在 WSL Ubuntu 交叉编译 Windows EXE

Tauri 在 Linux 上通过 `cargo-xwin` 交叉编译 MSVC 目标。建议把源码放在 WSL 的 Linux 文件系统（例如 `~/src/totp-desk`），避免直接在 `/mnt/c` 下编译。

1. 安装系统依赖：

```bash
sudo apt update
sudo apt install -y curl build-essential clang lld llvm nsis pkg-config libssl-dev
```

2. 如果当前 Cargo 来自 `apt`，先删除旧版本并使用 rustup：

```bash
sudo apt remove -y cargo rustc
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup update
```

3. 安装交叉编译目标和 runner：

```bash
rustup target add x86_64-pc-windows-msvc
cargo install --locked cargo-xwin
```

4. 安装前端依赖并只构建 NSIS 安装程序：

```bash
npm ci
npm run tauri build -- \
  --runner cargo-xwin \
  --target x86_64-pc-windows-msvc \
  --bundles nsis
```

产物位于：

```text
src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/
```

跨平台构建不适合生成 MSI，也不会自动完成 Windows 代码签名。如果 `cargo-xwin` 遇到 SDK、NSIS 或原生依赖问题，优先改用 Windows 原生环境或 Windows GitHub Actions runner。

## JSON 备份格式

导出的 JSON 包含明文 Base32 Seed，便于迁移，但必须按敏感凭据保管。本地数据库 `accounts.dat` 使用当前 Windows 用户的 DPAPI 加密，不能直接复制给其他用户解密。

## 安全边界

本应用是离线验证器，不联网、不上传密钥。运行时仍需在内存中短暂持有解密后的 TOTP 密钥；进程退出时由 `zeroize` 擦除 Rust 密钥缓冲区。操作系统崩溃转储、已被入侵的管理员账户等场景不在本应用的防护范围内。
