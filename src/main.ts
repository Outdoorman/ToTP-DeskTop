import { invoke } from "@tauri-apps/api/core";
import "./styles.css";

type Algorithm = "SHA1" | "SHA256" | "SHA512";

interface AccountView {
  id: string;
  label: string;
  issuer: string;
  algorithm: Algorithm;
  digits: number;
  period: number;
  code: string;
  remaining: number;
}

interface AddInput {
  secret: string;
  label: string;
  issuer: string;
  algorithm: Algorithm;
  digits: number;
  period: number;
}

interface UpdateInput extends AddInput { id: string; }

const icon = (name: "plus" | "search" | "shield" | "key" | "help" | "back" | "edit" | "screen" | "copy" | "trash" | "export" | "qr" | "file" | "text" | "x" | "camera") => {
  const paths: Record<typeof name, string> = {
    plus: '<path d="M12 5v14M5 12h14"/>',
    search: '<circle cx="11" cy="11" r="7"/><path d="m20 20-4-4"/>',
    shield: '<path d="M12 3 5.5 6v5c0 4.5 2.7 8.1 6.5 10 3.8-1.9 6.5-5.5 6.5-10V6L12 3Z"/><path d="m9.2 12 1.8 1.8 3.8-4"/>',
    key: '<circle cx="9" cy="10" r="4"/><path d="m12 13 7 7m-3-3 2-2m-5-1.5 2-2"/>',
    help: '<circle cx="12" cy="12" r="9"/><path d="M9.8 9a2.4 2.4 0 1 1 3.7 2c-1 .65-1.5 1.15-1.5 2.5M12 17h.01"/>',
    back: '<path d="m15 18-6-6 6-6"/>',
    edit: '<path d="M13.5 6.5 17.5 10.5M4 20l4.5-1 10-10a2.8 2.8 0 0 0-4-4l-10 10L4 20Z"/>',
    screen: '<rect x="3" y="4" width="18" height="13" rx="2"/><path d="M8 21h8M12 17v4M7 8h3M7 8v3"/>',
    copy: '<rect x="8" y="8" width="11" height="11" rx="2"/><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"/>',
    trash: '<path d="M4 7h16M9 7V4h6v3M7 7l1 14h8l1-14M10 11v6M14 11v6"/>',
    export: '<path d="M12 3v12m0-12 4 4m-4-4L8 7"/><path d="M5 13v7h14v-7"/>',
    qr: '<path d="M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h2v2h-2zM18 14h2v6h-6v-2M18 18h2"/>',
    file: '<path d="M6 3h8l4 4v14H6z"/><path d="M14 3v5h5M9 13h6M9 17h6"/>',
    text: '<path d="M4 6V4h16v2M12 4v16M8 20h8"/>',
    x: '<path d="m6 6 12 12M18 6 6 18"/>',
    camera: '<path d="M4 7h3l2-3h6l2 3h3v12H4z"/><circle cx="12" cy="13" r="4"/>',
  };
  return `<svg aria-hidden="true" viewBox="0 0 24 24">${paths[name]}</svg>`;
};

document.querySelector<HTMLDivElement>("#app")!.innerHTML = `
  <div class="app-shell">
    <header class="app-header">
      <div class="profile-badge" title="数据已在本机加密">${icon("shield")}<i></i></div>
      <div class="app-title"><h1 id="page-title">2FA 代码</h1><p><span id="nav-count">0</span> 个账号 · 本机加密</p></div>
      <button class="header-help" title="验证码每 30 秒自动更新" aria-label="帮助">${icon("help")}</button>
    </header>

    <main class="workspace">
      <section id="codes-view">
        <div class="toolbar">
          <label class="search">${icon("search")}<input id="search" type="search" placeholder="搜索" autocomplete="off" /></label>
          <div class="toolbar-actions">
            <button class="tool-button" id="transfer-button" title="导入与备份" aria-label="导入与备份">${icon("file")}<i>${icon("plus")}</i></button>
            <button class="tool-button" id="qr-quick" title="扫描二维码" aria-label="扫描二维码">${icon("qr")}</button>
            <button class="tool-button primary-icon" id="add-button" title="添加账号" aria-label="添加账号">${icon("plus")}</button>
          </div>
        </div>
        <div class="account-grid" id="account-grid"></div>
        <div class="empty hidden" id="empty-state">
          <div class="empty-icon">${icon("key")}</div><h2>还没有验证码</h2><p>添加 TOTP 密钥，或从二维码、JSON 备份导入。</p><button class="primary" data-open-add>${icon("plus")} 添加第一个账号</button>
        </div>
      </section>

      <section class="hidden" id="transfer-view">
        <div class="section-head"><button class="back-button" id="back-to-codes" aria-label="返回">${icon("back")}</button><div><h2>导入与备份</h2><p>迁移账号或创建离线备份</p></div></div>
        <div class="transfer-grid">
          <button class="transfer-card" id="text-import"><span class="transfer-icon violet">${icon("text")}</span><div><b>文本导入</b><small>Seed 或 otpauth://，每行一个</small></div></button>
          <button class="transfer-card" id="qr-import"><span class="transfer-icon mint">${icon("qr")}</span><div><b>扫描二维码</b><small>摄像头、图片或粘贴截图</small></div></button>
          <button class="transfer-card" id="json-import"><span class="transfer-icon blue">${icon("file")}</span><div><b>导入 JSON</b><small>TOTP Desk 明文备份文件</small></div></button>
          <button class="transfer-card" id="export-all"><span class="transfer-icon amber">${icon("export")}</span><div><b>导出全部</b><small>保存可迁移的 JSON 备份</small></div></button>
        </div>
        <div class="warning"><b>备份安全提醒</b><p>导出的 JSON 包含可生成验证码的明文密钥。请保存到加密磁盘或受信任的密码管理器，不要通过聊天工具传输。</p></div>
      </section>
    </main>
    <footer><span class="clock-dot"></span><span>验证码实时更新</span><span class="secure-footer">${icon("shield")} Windows DPAPI</span></footer>
  </div>

  <dialog id="add-dialog">
    <form method="dialog" id="add-form">
      <div class="dialog-head"><div><h2>添加 TOTP</h2><p>手动输入服务提供的密钥</p></div><button type="button" class="icon-btn close">${icon("x")}</button></div>
      <label>密钥 Seed<input id="seed" name="secret" required spellcheck="false" autocomplete="off" placeholder="JBSWY3DPEHPK3PXP" /></label>
      <div class="two-col"><label>服务名称<input id="issuer" name="issuer" placeholder="例如 GitHub" /></label><label>账号名称<input id="label" name="label" placeholder="name@example.com" /></label></div>
      <div class="three-col"><label>算法<select id="algorithm"><option>SHA1</option><option>SHA256</option><option>SHA512</option></select></label><label>位数<select id="digits"><option value="6">6 位</option><option value="8">8 位</option></select></label><label>周期<input id="period" type="number" value="30" min="15" max="120" /></label></div>
      <div class="dialog-actions"><button type="button" class="secondary close">取消</button><button class="primary" value="default">保存账号</button></div>
    </form>
  </dialog>

  <dialog id="edit-dialog">
    <form method="dialog" id="edit-form">
      <div class="dialog-head"><div><h2>编辑 TOTP</h2><p>留空 Seed 将继续使用原密钥</p></div><button type="button" class="icon-btn close">${icon("x")}</button></div>
      <input id="edit-id" type="hidden" />
      <label>新密钥 Seed（可选）<input id="edit-seed" spellcheck="false" autocomplete="off" placeholder="不修改请留空" /></label>
      <div class="two-col"><label>服务名称<input id="edit-issuer" placeholder="例如 GitHub" /></label><label>账号名称<input id="edit-label" placeholder="name@example.com" /></label></div>
      <div class="three-col"><label>算法<select id="edit-algorithm"><option>SHA1</option><option>SHA256</option><option>SHA512</option></select></label><label>位数<select id="edit-digits"><option value="6">6 位</option><option value="8">8 位</option></select></label><label>周期<input id="edit-period" type="number" value="30" min="15" max="120" /></label></div>
      <div class="dialog-actions"><button type="button" class="secondary close">取消</button><button class="primary" value="default">保存修改</button></div>
    </form>
  </dialog>

  <dialog id="text-dialog">
    <form method="dialog" id="text-form">
      <div class="dialog-head"><div><h2>文本导入</h2><p>每行一个 Seed、otpauth:// 或 Google 迁移链接</p></div><button type="button" class="icon-btn close">${icon("x")}</button></div>
      <textarea id="import-text" rows="9" required spellcheck="false" placeholder="otpauth://totp/GitHub:user@example.com?secret=...&#10;JBSWY3DPEHPK3PXP"></textarea>
      <div class="dialog-actions"><button type="button" class="secondary close">取消</button><button class="primary">开始导入</button></div>
    </form>
  </dialog>

  <dialog id="qr-dialog" class="qr-dialog">
    <div class="dialog-head"><div><h2>扫描二维码</h2><p>支持标准 TOTP 与 Google Authenticator 迁移码</p></div><button type="button" class="icon-btn close">${icon("x")}</button></div>
    <div class="camera-box"><video id="camera" playsinline muted></video><div class="scan-frame"><i></i><i></i><i></i><i></i></div><div class="camera-placeholder" id="camera-placeholder">${icon("camera")}<span>点击下方按钮启用摄像头</span></div></div>
    <canvas id="capture" class="hidden"></canvas>
    <div class="qr-actions"><button class="secondary" id="start-camera">${icon("camera")} 摄像头</button><button class="secondary" id="capture-screen">${icon("screen")} 截取屏幕</button><label class="secondary">${icon("file")} 选择图片<input class="hidden" id="qr-file" type="file" accept="image/png,image/jpeg,image/webp" /></label></div>
    <p class="paste-tip">截屏时请选择包含二维码的屏幕或窗口，也可按 Ctrl+V 粘贴截图</p>
  </dialog>

  <div id="toast" role="status" aria-live="polite"></div>
`;

const $ = <T extends Element>(selector: string) => document.querySelector<T>(selector)!;
const grid = $("#account-grid") as HTMLDivElement;
const empty = $("#empty-state") as HTMLDivElement;
const addDialog = $("#add-dialog") as HTMLDialogElement;
const editDialog = $("#edit-dialog") as HTMLDialogElement;
const textDialog = $("#text-dialog") as HTMLDialogElement;
const qrDialog = $("#qr-dialog") as HTMLDialogElement;
const video = $("#camera") as HTMLVideoElement;
const canvas = $("#capture") as HTMLCanvasElement;

let accounts: AccountView[] = [];
let query = "";
let stream: MediaStream | null = null;
let scanTimer: number | null = null;
let busy = false;
let scanningFrame = false;
let lastTickSecond = 0;
let lastBoundaryRefresh = 0;

function escapeHtml(value: string): string {
  return value.replace(/[&<>'"]/g, (char) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", "'": "&#39;", '"': "&quot;" })[char]!);
}

function palette(value: string): number {
  let hash = 0;
  for (const char of value) hash = (hash * 31 + char.charCodeAt(0)) | 0;
  return Math.abs(hash) % 6;
}

function render(): void {
  const shown = accounts.filter((a) => `${a.issuer} ${a.label}`.toLowerCase().includes(query));
  $("#nav-count").textContent = String(accounts.length);
  empty.classList.toggle("hidden", accounts.length !== 0 || query.length !== 0);
  grid.innerHTML = shown.length ? shown.map((a) => {
    const split = Math.ceil(a.code.length / 2);
    const progress = Math.max(0, Math.min(100, (a.remaining / a.period) * 100));
    return `<article class="account-card" data-id="${escapeHtml(a.id)}" data-copy="${escapeHtml(a.code)}" role="button" tabindex="0" aria-label="复制 ${escapeHtml(a.issuer || a.label || "TOTP")} 验证码">
      <div class="card-main">
        <span class="avatar p${palette(a.issuer || a.label)}">${icon("key")}</span>
        <div class="identity"><b>${escapeHtml(a.issuer || "未命名服务")}</b><span>${escapeHtml(a.label || "TOTP 账号")}</span></div>
        <div class="code-area"><span class="code"><span>${a.code.slice(0, split)}</span><span>${a.code.slice(split)}</span></span><span class="copy-hint">${icon("copy")} 点击任意位置复制</span></div>
      </div>
      <div class="countdown"><div class="bar"><i data-progress style="width:${progress}%"></i></div><span data-remaining>${a.remaining}s</span></div>
      <div class="card-actions"><span>${a.algorithm} · ${a.digits} 位</span><button data-edit="${escapeHtml(a.id)}" title="编辑账号" aria-label="编辑账号">${icon("edit")}</button><button data-export="${escapeHtml(a.id)}" title="导出此账号" aria-label="导出此账号">${icon("export")}</button><button class="danger" data-delete="${escapeHtml(a.id)}" title="删除账号" aria-label="删除账号">${icon("trash")}</button></div>
    </article>`;
  }).join("") : query ? '<div class="no-results">没有匹配的账号</div>' : "";
}

function showToast(message: string, kind: "ok" | "error" = "ok"): void {
  const toast = $("#toast") as HTMLDivElement;
  toast.textContent = message;
  toast.className = `show ${kind}`;
  window.setTimeout(() => toast.className = "", 2600);
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

async function refresh(silent = false): Promise<void> {
  try {
    accounts = await invoke<AccountView[]>("list_accounts");
    render();
  } catch (error) {
    if (!silent) showToast(errorMessage(error), "error");
  }
}

function tickCountdowns(): void {
  const now = Math.floor(Date.now() / 1000);
  if (now === lastTickSecond) return;
  lastTickSecond = now;
  let atBoundary = false;
  for (const account of accounts) {
    const remaining = account.period - now % account.period;
    account.remaining = remaining;
    const card = grid.querySelector<HTMLElement>(`.account-card[data-id="${account.id}"]`);
    const label = card?.querySelector<HTMLElement>("[data-remaining]");
    const progress = card?.querySelector<HTMLElement>("[data-progress]");
    if (label) label.textContent = `${remaining}s`;
    if (progress) progress.style.width = `${(remaining / account.period) * 100}%`;
    if (now % account.period === 0) atBoundary = true;
  }
  if (atBoundary && now !== lastBoundaryRefresh && !busy) {
    lastBoundaryRefresh = now;
    void refresh(true);
  }
}

async function withBusy(action: () => Promise<void>): Promise<void> {
  if (busy) return;
  busy = true;
  document.body.classList.add("busy");
  try { await action(); } catch (error) { showToast(errorMessage(error), "error"); }
  finally { busy = false; document.body.classList.remove("busy"); }
}

async function copyCode(code: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(code);
    showToast("验证码已复制");
  } catch {
    const input = document.createElement("textarea");
    input.value = code; document.body.appendChild(input); input.select();
    document.execCommand("copy"); input.remove(); showToast("验证码已复制");
  }
}

function closeDialog(dialog: HTMLDialogElement): void {
  if (dialog === qrDialog) stopCamera();
  dialog.close();
}

function openEdit(id: string): void {
  const account = accounts.find((item) => item.id === id);
  if (!account) { showToast("账号不存在，请刷新后重试", "error"); return; }
  ($("#edit-id") as HTMLInputElement).value = account.id;
  ($("#edit-seed") as HTMLInputElement).value = "";
  ($("#edit-issuer") as HTMLInputElement).value = account.issuer;
  ($("#edit-label") as HTMLInputElement).value = account.label;
  ($("#edit-algorithm") as HTMLSelectElement).value = account.algorithm;
  ($("#edit-digits") as HTMLSelectElement).value = String(account.digits);
  ($("#edit-period") as HTMLInputElement).value = String(account.period);
  editDialog.showModal();
}

document.addEventListener("click", (event) => {
  const target = (event.target as HTMLElement).closest<HTMLElement>("button, [data-open-add], .account-card[data-copy]");
  if (!target) return;
  if (target.matches("[data-open-add], #add-button")) addDialog.showModal();
  if (target.classList.contains("close")) closeDialog(target.closest("dialog") as HTMLDialogElement);
  const copy = target.dataset.copy; if (copy) void copyCode(copy);
  const editId = target.dataset.edit; if (editId) openEdit(editId);
  const exportId = target.dataset.export; if (exportId) void exportAccounts(exportId);
  const deleteId = target.dataset.delete;
  if (deleteId && window.confirm("确定删除这个 TOTP 账号？此操作无法撤销。")) void withBusy(async () => {
    await invoke("delete_account", { id: deleteId }); await refresh(); showToast("账号已删除");
  });
});

grid.addEventListener("keydown", (event) => {
  if (event.key !== "Enter" && event.key !== " ") return;
  const card = (event.target as HTMLElement).closest<HTMLElement>(".account-card[data-copy]");
  if (!card || (event.target as HTMLElement).closest("button")) return;
  event.preventDefault();
  if (card.dataset.copy) void copyCode(card.dataset.copy);
});

function showCodes(codes: boolean): void {
  $("#codes-view").classList.toggle("hidden", !codes);
  $("#transfer-view").classList.toggle("hidden", codes);
  $("#page-title").textContent = codes ? "2FA 代码" : "安全中心";
}

$("#transfer-button").addEventListener("click", () => showCodes(false));
$("#back-to-codes").addEventListener("click", () => showCodes(true));
$("#qr-quick").addEventListener("click", () => qrDialog.showModal());

$("#search").addEventListener("input", (event) => { query = (event.target as HTMLInputElement).value.trim().toLowerCase(); render(); });
$("#text-import").addEventListener("click", () => textDialog.showModal());
$("#qr-import").addEventListener("click", () => qrDialog.showModal());

$("#add-form").addEventListener("submit", (event) => {
  event.preventDefault();
  const input: AddInput = {
    secret: ($("#seed") as HTMLInputElement).value,
    issuer: ($("#issuer") as HTMLInputElement).value.trim(),
    label: ($("#label") as HTMLInputElement).value.trim(),
    algorithm: ($("#algorithm") as HTMLSelectElement).value as Algorithm,
    digits: Number(($("#digits") as HTMLSelectElement).value),
    period: Number(($("#period") as HTMLInputElement).value),
  };
  void withBusy(async () => {
    await invoke("add_account", { input });
    (event.target as HTMLFormElement).reset(); ($("#period") as HTMLInputElement).value = "30";
    closeDialog(addDialog); await refresh(); showToast("账号已添加");
  });
});

$("#edit-form").addEventListener("submit", (event) => {
  event.preventDefault();
  const input: UpdateInput = {
    id: ($("#edit-id") as HTMLInputElement).value,
    secret: ($("#edit-seed") as HTMLInputElement).value,
    issuer: ($("#edit-issuer") as HTMLInputElement).value.trim(),
    label: ($("#edit-label") as HTMLInputElement).value.trim(),
    algorithm: ($("#edit-algorithm") as HTMLSelectElement).value as Algorithm,
    digits: Number(($("#edit-digits") as HTMLSelectElement).value),
    period: Number(($("#edit-period") as HTMLInputElement).value),
  };
  void withBusy(async () => {
    await invoke("update_account", { input });
    closeDialog(editDialog); await refresh(); showToast("账号已更新");
  });
});

$("#text-form").addEventListener("submit", (event) => {
  event.preventDefault();
  const text = ($("#import-text") as HTMLTextAreaElement).value;
  void withBusy(async () => {
    const count = await invoke<number>("import_text", { text });
    (event.target as HTMLFormElement).reset(); closeDialog(textDialog); await refresh(); showToast(`已导入 ${count} 个账号`);
  });
});

$("#json-import").addEventListener("click", () => void withBusy(async () => {
  const count = await invoke<number>("import_json_dialog");
  if (count > 0) { await refresh(); showToast(`已导入 ${count} 个账号`); }
}));

async function exportAccounts(id?: string): Promise<void> {
  if (!window.confirm("导出的 JSON 包含明文 TOTP 密钥。确定继续吗？")) return;
  await withBusy(async () => {
    const saved = await invoke<boolean>("export_accounts", { id: id ?? null });
    if (saved) showToast(id ? "账号已导出" : "完整备份已导出");
  });
}
$("#export-all").addEventListener("click", () => void exportAccounts());

async function scanBytes(bytes: Uint8Array): Promise<void> {
  if (busy) return;
  await withBusy(async () => {
    const count = await invoke<number>("import_qr", { bytes: Array.from(bytes) });
    closeDialog(qrDialog); await refresh(); showToast(`已从二维码导入 ${count} 个账号`);
  });
}

$("#qr-file").addEventListener("change", (event) => {
  const input = event.target as HTMLInputElement; const file = input.files?.[0]; if (!file) return;
  void file.arrayBuffer().then((buffer) => scanBytes(new Uint8Array(buffer))).finally(() => input.value = "");
});

async function captureScreenQr(): Promise<void> {
  if (!navigator.mediaDevices?.getDisplayMedia) { showToast("当前系统不支持屏幕截取", "error"); return; }
  let screenStream: MediaStream | null = null;
  try {
    stopCamera();
    screenStream = await navigator.mediaDevices.getDisplayMedia({ video: true, audio: false });
    const preview = document.createElement("video");
    preview.muted = true;
    preview.playsInline = true;
    preview.srcObject = screenStream;
    await new Promise<void>((resolve, reject) => {
      preview.onloadedmetadata = () => resolve();
      preview.onerror = () => reject(new Error("无法读取截屏画面"));
    });
    await preview.play();
    const scale = Math.min(1, 1920 / Math.max(preview.videoWidth, preview.videoHeight));
    canvas.width = Math.max(1, Math.round(preview.videoWidth * scale));
    canvas.height = Math.max(1, Math.round(preview.videoHeight * scale));
    canvas.getContext("2d", { alpha: false })?.drawImage(preview, 0, 0, canvas.width, canvas.height);
    const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/png"));
    if (!blob) throw new Error("截屏生成失败");
    await scanBytes(new Uint8Array(await blob.arrayBuffer()));
  } catch (error) {
    const name = error instanceof DOMException ? error.name : "";
    if (name !== "NotAllowedError" && name !== "AbortError") showToast(`截屏读取失败：${errorMessage(error)}`, "error");
  } finally {
    screenStream?.getTracks().forEach((track) => track.stop());
  }
}

$("#capture-screen").addEventListener("click", () => void captureScreenQr());

document.addEventListener("paste", (event) => {
  if (!qrDialog.open) return;
  const file = Array.from(event.clipboardData?.files ?? []).find((item) => item.type.startsWith("image/"));
  if (file) { event.preventDefault(); void file.arrayBuffer().then((buffer) => scanBytes(new Uint8Array(buffer))); }
});

function stopCamera(): void {
  if (scanTimer !== null) window.clearInterval(scanTimer);
  scanTimer = null; stream?.getTracks().forEach((track) => track.stop()); stream = null; video.srcObject = null;
  $("#camera-placeholder").classList.remove("hidden");
}

async function captureFrame(): Promise<void> {
  if (!stream || busy || scanningFrame || video.readyState < 2) return;
  scanningFrame = true;
  try {
    canvas.width = Math.min(video.videoWidth, 720);
    canvas.height = Math.round(video.videoHeight * (canvas.width / video.videoWidth));
    canvas.getContext("2d", { alpha: false })?.drawImage(video, 0, 0, canvas.width, canvas.height);
    const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/jpeg", 0.82));
    if (!blob) return;
    const count = await invoke<number>("try_import_qr", { bytes: Array.from(new Uint8Array(await blob.arrayBuffer())) });
    if (count > 0) { stopCamera(); closeDialog(qrDialog); await refresh(); showToast(`已从二维码导入 ${count} 个账号`); }
  } catch { /* no QR in this frame */ }
  finally { scanningFrame = false; }
}

$("#start-camera").addEventListener("click", () => void (async () => {
  try {
    stopCamera(); stream = await navigator.mediaDevices.getUserMedia({ video: { facingMode: "environment", width: { ideal: 1280 } }, audio: false });
    video.srcObject = stream; await video.play(); $("#camera-placeholder").classList.add("hidden");
    scanTimer = window.setInterval(() => void captureFrame(), 900);
  } catch (error) { showToast(`无法启用摄像头：${errorMessage(error)}`, "error"); }
})());

qrDialog.addEventListener("close", stopCamera);
window.addEventListener("beforeunload", stopCamera);
window.setInterval(tickCountdowns, 250);
void refresh();
