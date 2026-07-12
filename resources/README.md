# resources/ —— 隧道程序 frpc 二进制

乘风连接器启动时从这里取 frpc 子程序，出向连专机 frps。

## 放哪：`resources/bin/`

按目标平台放对应 frpc 二进制（**版本必须与专机 frps 一致**，见下方版本号）：

| 目标平台 | 文件名 | 来源 |
|---|---|---|
| macOS Apple Silicon (M1/M2…) | `resources/bin/frpc` (darwin arm64) | frp release `frp_x.x.x_darwin_arm64.tar.gz` 内的 `frpc` |
| macOS Intel | `resources/bin/frpc` (darwin amd64) | `frp_x.x.x_darwin_amd64.tar.gz` |
| Windows | `resources/bin/frpc.exe` (windows amd64) | `frp_x.x.x_windows_amd64.zip` 内的 `frpc.exe` |

> 同一平台构建只放该平台的 frpc 即可（`tauri.conf.json` 的 `resources: ["resources/bin/*"]` 会把它打进包）。

## frp 版本

当前对齐：**frp v0.69.1**（与专机 `frps` 镜像 `snowdreamtech/frps:latest` 对齐；若专机换版本，这里同步换，CI 里改 `build.yml` 的 `FRP_VER`）。

下载：https://github.com/fatedier/frp/releases

## 一键拉取（macOS 构建示例）

```bash
cd connector/resources
mkdir -p bin
# 选与当前 Mac 匹配的架构
ARCH=$(uname -m)              # arm64 或 x86_64
case "$ARCH" in
  arm64)   FRP_ARCH=darwin_arm64;;
  x86_64)  FRP_ARCH=darwin_amd64;;
esac
VER=0.69.1
curl -L "https://github.com/fatedier/frp/releases/download/v${VER}/frp_${VER}_${FRP_ARCH}.tar.gz" -o /tmp/frp.tgz
tar -xzf /tmp/frp.tgz -C /tmp "frp_${VER}_${FRP_ARCH}/frpc"
cp /tmp/frp_${VER}_${FRP_ARCH}/frpc bin/frpc
chmod +x bin/frpc
```

Windows 构建在 Windows 机器上同理，放 `bin/frpc.exe`。

## 注意

- **版本对齐**：frpc 与 frps 大版本必须一致（都用 0.6x），否则握手失败。
- **可执行权限**：mac/linux 打包后可能丢 x 位，连接器启动时会主动 `chmod 755`（见 `src/main.rs` connect）。
- frpc 二进制不进 git（体积大），构建前由脚本拉取（见上方）。`.gitignore` 应含 `connector/resources/bin/`。
