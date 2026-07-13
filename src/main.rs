#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// 乘风连接器（试用阶段）· Rust 主体
// 流程：解析连接码(base64 JSON) → 生成 frpc.toml → 拉起 frpc 子进程出向连专机 frps → 维护状态
// 详见 docs/deploy/乘风连接器试用部署方案.md

use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

use base64::engine::general_purpose::URL_SAFE;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

/// 连接码解析后的凭证（与后端 trial_handler.provision 生成的 payload 一一对应）
#[derive(Debug, Deserialize)]
struct Payload {
    tenant: String,
    frps: String, // "host:443"
    remote_port: u16,
    frps_token: String,
    socks_user: String,
    socks_pass: String,
    #[serde(default)]
    #[allow(dead_code)]
    expires_at: String,
    #[serde(default)]
    #[allow(dead_code)]
    sig: String, // 连接器不校验 sig（服务端 frps auth.token 把关），仅解析
}

#[derive(Serialize, Clone)]
struct Status {
    connected: bool,
    tenant: Option<String>,
    expires_at: Option<String>,
    error: Option<String>,
}

#[derive(Clone)]
struct PayloadInfo {
    tenant: String,
    expires_at: String,
}

struct AppState {
    child: Mutex<Option<Child>>,
    info: Mutex<Option<PayloadInfo>>,
}

/// base64 解码连接码 → JSON → Payload
fn parse_code(code: &str) -> Result<Payload, String> {
    let code = code.trim();
    let code = code
        .strip_prefix("data:text/plain;base64,")
        .unwrap_or(code);
    let bytes = URL_SAFE
        .decode(code.as_bytes())
        .map_err(|e| format!("连接码格式错误（base64 解码失败）：{e}"))?;
    let json = String::from_utf8(bytes).map_err(|e| format!("连接码编码异常：{e}"))?;
    serde_json::from_str::<Payload>(&json)
        .map_err(|e| format!("连接码内容异常：{e}"))
}

/// 生成 frpc.toml 内容（frp v2(toml) 格式；版本须与专机 frps 对齐）
fn build_frpc_config(p: &Payload) -> String {
    let (host, port) = match p.frps.rsplit_once(':') {
        Some((h, prt)) => (h.to_string(), prt.to_string()),
        None => (p.frps.clone(), "443".to_string()),
    };
    format!(
        r#"# 由乘风连接器自动生成（试用隧道客户端，请勿手改）
serverAddr = "{host}"
serverPort = {port}
auth.token = "{token}"
transport.tls.enable = true
loginFailExit = false

[[proxies]]
name = "trial_socks5_{rport}"
type = "tcp"
remotePort = {rport}
[proxies.plugin]
type = "socks5"
username = "{user}"
password = "{pass}"
"#,
        host = host,
        port = port,
        token = p.frps_token,
        rport = p.remote_port,
        user = p.socks_user,
        pass = p.socks_pass,
    )
}

fn frpc_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "frpc.exe"
    } else {
        "frpc"
    }
}

#[tauri::command]
fn connect(
    code: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Status, String> {
    // 先断开旧连接
    if let Some(mut child) = state.child.lock().unwrap().take() {
        let _ = child.kill();
        let _ = child.wait();
    }

    let payload = parse_code(&code)?;

    // 写 frpc.toml 到 app config 目录
    let cfg_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("定位配置目录失败：{e}"))?;
    std::fs::create_dir_all(&cfg_dir).map_err(|e| format!("创建配置目录失败：{e}"))?;
    let cfg_path = cfg_dir.join("frpc.toml");
    std::fs::write(&cfg_path, build_frpc_config(&payload))
        .map_err(|e| format!("写 frpc 配置失败：{e}"))?;

    // frpc 二进制（resources/bin/frpc[.exe]，打包进 resource_dir）
    let res_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("定位资源目录失败：{e}"))?;
    let frpc_path = res_dir
        .join("resources")
        .join("bin")
        .join(frpc_binary_name());
    if !frpc_path.exists() {
        return Err(format!(
            "未找到隧道程序 frpc（{}）。请重新下载完整版连接器，或联系乘风支持。",
            frpc_path.display()
        ));
    }
    // 给 frpc 可执行权限（mac/linux 打包后可能丢 x bit）
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(&frpc_path) {
            let mut perm = meta.permissions();
            perm.set_mode(0o755);
            let _ = std::fs::set_permissions(&frpc_path, perm);
        }
    }

    let child = Command::new(&frpc_path)
        .arg("-c")
        .arg(&cfg_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("启动 frpc 失败：{e}"))?;

    let info = PayloadInfo {
        tenant: payload.tenant.clone(),
        expires_at: payload.expires_at.clone(),
    };
    *state.info.lock().unwrap() = Some(info.clone());
    *state.child.lock().unwrap() = Some(child);

    Ok(Status {
        connected: true,
        tenant: Some(info.tenant),
        expires_at: Some(info.expires_at),
        error: None,
    })
}

#[tauri::command]
fn disconnect(state: State<'_, AppState>) -> Result<Status, String> {
    if let Some(mut child) = state.child.lock().unwrap().take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    *state.info.lock().unwrap() = None;
    Ok(Status {
        connected: false,
        tenant: None,
        expires_at: None,
        error: None,
    })
}

#[tauri::command]
fn status(state: State<'_, AppState>) -> Status {
    let mut guard = state.child.lock().unwrap();
    if let Some(child) = guard.as_mut() {
        match child.try_wait() {
            Ok(None) => {
                let info = state.info.lock().unwrap();
                return Status {
                    connected: true,
                    tenant: info.as_ref().map(|i| i.tenant.clone()),
                    expires_at: info.as_ref().map(|i| i.expires_at.clone()),
                    error: None,
                };
            }
            Ok(Some(_)) => {
                *guard = None;
                *state.info.lock().unwrap() = None;
                return Status {
                    connected: false,
                    tenant: None,
                    expires_at: None,
                    error: Some("连接已断开（隧道程序退出，可能是网络中断或试用已到期）".into()),
                };
            }
            Err(_) => {}
        }
    }
    Status {
        connected: false,
        tenant: None,
        expires_at: None,
        error: None,
    }
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            child: Mutex::new(None),
            info: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![connect, disconnect, status])
        .run(tauri::generate_context!())
        .expect("乘风连接器启动失败");
}
