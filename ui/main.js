// 乘风连接器 · 前端逻辑（vanilla JS，Tauri 2 withGlobalTauri）
const __T = window.__TAURI__;
const invoke = (__T && ((__T.core && __T.core.invoke) || __T.invoke)) || null;

const $code = document.getElementById("code");
const $connect = document.getElementById("connect");
const $disconnect = document.getElementById("disconnect");
const $status = document.getElementById("status");
const $meta = document.getElementById("meta");

function setStatus(s) {
  if (!s || (!s.connected && !s.error)) {
    $status.textContent = "未连接";
    $status.className = "status idle";
    $meta.textContent = "";
    return;
  }
  if (s.connected) {
    $status.textContent = "● 已连接";
    $status.className = "status ok";
    $meta.textContent =
      (s.tenant ? "试用：" + s.tenant : "") +
      (s.expires_at ? "　到期：" + s.expires_at : "");
  } else {
    $status.textContent = "● " + (s.error || "已断开");
    $status.className = "status err";
    $meta.textContent = "";
  }
}

$connect.addEventListener("click", async () => {
  if (!invoke) {
    $status.textContent = "运行环境异常（无 Tauri 桥）";
    $status.className = "status err";
    return;
  }
  const code = $code.value.trim();
  if (!code) {
    $status.textContent = "请先粘贴连接码";
    $status.className = "status err";
    return;
  }
  $connect.disabled = true;
  $status.textContent = "连接中…";
  $status.className = "status busy";
  try {
    const s = await invoke("connect", { code });
    setStatus(s);
  } catch (e) {
    $status.textContent = "连接失败：" + (e && (e.message || e));
    $status.className = "status err";
  } finally {
    $connect.disabled = false;
  }
});

$disconnect.addEventListener("click", async () => {
  if (!invoke) return;
  try {
    await invoke("disconnect");
  } catch (e) {
    /* ignore */
  }
  setStatus(null);
});

// 轮询状态（检测 frpc 进程退出 / 到期断开）
async function tick() {
  if (!invoke) return;
  try {
    const s = await invoke("status");
    setStatus(s);
  } catch (e) {
    /* ignore */
  }
}
setInterval(tick, 3000);
tick();
