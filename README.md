# 乘风连接器（Chengfeng Connector）

试用阶段客户端：Tauri 桌面程序，客户双击 → 粘贴连接码 → 出向 socks5 隧道连阿里云专机 frps，让专机能访问客户内网被测系统。

> 完整方案见 [`../docs/deploy/乘风连接器试用部署方案.md`](../docs/deploy/乘风连接器试用部署方案.md)，操作手册见 [`../docs/deploy/乘风连接器-试用上线操作手册.md`](../docs/deploy/乘风连接器-试用上线操作手册.md)。

## 目录结构

```
connector/
├── Cargo.toml          # Rust 依赖（tauri 2 / serde / base64）
├── build.rs            # tauri-build
├── tauri.conf.json     # 窗口/打包/resources/图标 配置
├── src/main.rs         # Rust 主体：解析连接码 → 生成 frpc.toml → 拉起 frpc → 维护状态
├── ui/                 # 前端（vanilla JS，withGlobalTauri 直调 invoke）
│   ├── index.html      # 连接码输入 + 状态展示
│   ├── main.js
│   └── style.css
├── resources/
│   ├── README.md       # frpc 二进制拉取说明（版本须与专机 frps 对齐）
│   └── bin/            # frpc 二进制（构建前拉取，不进 git）
└── icons/              # 应用图标（由 `cargo tauri icon icons/source.png` 生成全套）
```

## 工作原理

1. 管理员在乘风前端「试用连接器」页开通试用，得到一串 **连接码**（base64 JSON，含 frps 地址/端口/socks5 凭证）。
2. 客户在本程序粘贴连接码 → 点「连接」。
3. 程序 base64 解码连接码 → 生成 `frpc.toml` → 拉起内嵌 frpc 子进程 → frpc 出向连专机 frps(443+TLS)。
4. 专机的 playwright 浏览器经 socks5 代理（容器名 `chengfeng-frps`）访问客户被测系统。

## 构建（macOS）

```bash
# 0. 装 Rust（若未装）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
source "$HOME/.cargo/env"

# 1. 装 tauri-cli
cargo install tauri-cli --version "^2.0"

# 2. 拉 frpc 二进制到 resources/bin/（见 resources/README.md，版本对齐专机 frps）

# 3. 生成图标全套（从 source.png 占位；换品牌 logo 后重跑）
cargo tauri icon icons/source.png

# 4. 构建（产物在 target/release/bundle/）
cargo tauri build
```

## 构建（GitHub Actions 云端，推荐）

本机不用装 Rust、不用找 Windows 机器——把本项目推到 GitHub 仓库，push 后云端自动产出 **Mac 通用包（M 芯片 + Intel 通吃）** 和 **Windows 安装包**：

1. 在 GitHub 新建一个仓库（私有即可），把整个 `connector/` 目录作为**仓库根**推上去。
2. 已内置工作流 [.github/workflows/build.yml](.github/workflows/build.yml)：push 到 `main` 自动触发；也可在 Actions 页手动「Run workflow」。
3. 跑完后在该 run 页 **Artifacts** 区下载：
   - `chengfeng-connector-macos-universal` → `.dmg`（M 芯片 + Intel 通用）
   - `chengfeng-connector-windows-x64` → `.exe` 安装包（NSIS）

> frpc 二进制由云端按 `build.yml` 里的 `FRP_VER` 自动拉取（当前 0.69.1，须与专机 frps 对齐；换版本改 `FRP_VER` 即可）。
> 产物未签名，客户安装时按 [客户安装说明](../docs/deploy/乘风连接器-客户安装说明.md) 做一次「右键打开 / 仍要运行」绕过即可。

## 开发调试

```bash
cargo tauri dev      # 热重载 ui，改动 main.rs/ui 即时生效
```

## 跨平台

| 平台 | 构建方式 | frpc | 产物 |
|---|---|---|---|
| macOS（M 芯片 + Intel 通用） | GitHub Actions `build.yml`（universal-apple-darwin） | lipo 合并 arm64+amd64 | `.dmg`（通用） |
| Windows x64 | GitHub Actions `build.yml`（nsis） | `frpc.exe`(windows amd64) | `.exe` 安装包 |
| 本机调试 / 自行构建 | `cargo tauri build` / `cargo tauri dev` | 见 resources/README.md | `.app` |

> yunfan 主仓在 GitLab，GitHub Actions 够不着；故 connector 需作为**独立 GitHub 仓库**才能用 CI 构建（详见上方「构建（GitHub Actions 云端）」）。

## 签名（发客户前必做，否则双击被拦）

- macOS：Apple Developer ID + `xcrun notarytool` 公证（个人账号即可，¥688/年）
- Windows：Azure Trusted Signing（云签名，SmartScreen 立即友好）

见操作手册第 6 步。
