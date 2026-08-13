mod crypto;
mod model;
mod otp;
mod storage;

use std::{fs, io::Cursor, path::PathBuf, sync::RwLock, time::{SystemTime, UNIX_EPOCH}};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use model::{parse_source, Account, AccountInput, AccountUpdateInput, AccountView, Backup, PersistedAccount};
use prost::Message;
use rqrr::PreparedImage;
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;

struct AppState {
    accounts: RwLock<Vec<Account>>,
    path: PathBuf,
}

fn lock_error() -> String { "内部数据锁异常，请重启应用".into() }

fn persist(state: &AppState, accounts: &[Account]) -> Result<(), String> {
    storage::save(&state.path, accounts)
}

#[tauri::command]
fn list_accounts(state: State<'_, AppState>) -> Result<Vec<AccountView>, String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_secs();
    let accounts = state.accounts.read().map_err(|_| lock_error())?;
    accounts.iter().map(|account| account.view(now)).collect()
}

#[tauri::command]
fn add_account(input: AccountInput, state: State<'_, AppState>) -> Result<(), String> {
    let account = Account::try_from(input)?;
    let mut accounts = state.accounts.write().map_err(|_| lock_error())?;
    accounts.push(account);
    if let Err(error) = persist(&state, &accounts) { accounts.pop(); return Err(error); }
    Ok(())
}

#[tauri::command]
fn update_account(input: AccountUpdateInput, state: State<'_, AppState>) -> Result<(), String> {
    let mut accounts = state.accounts.write().map_err(|_| lock_error())?;
    let index = accounts.iter().position(|account| account.id == input.id).ok_or("账号不存在")?;
    let updated = accounts[index].updated(input)?;
    let previous = std::mem::replace(&mut accounts[index], updated);
    if let Err(error) = persist(&state, &accounts) {
        accounts[index] = previous;
        return Err(error);
    }
    Ok(())
}

fn append_accounts(state: &AppState, incoming: Vec<Account>) -> Result<usize, String> {
    if incoming.is_empty() { return Err("没有找到可导入的 TOTP 账号".into()); }
    let count = incoming.len();
    let mut accounts = state.accounts.write().map_err(|_| lock_error())?;
    let old_len = accounts.len();
    accounts.extend(incoming);
    if let Err(error) = persist(state, &accounts) { accounts.truncate(old_len); return Err(error); }
    Ok(count)
}

#[tauri::command]
fn import_text(text: String, state: State<'_, AppState>) -> Result<usize, String> {
    let text = zeroize::Zeroizing::new(text);
    let mut imported = Vec::new();
    for (index, line) in text.lines().map(str::trim).filter(|line| !line.is_empty()).enumerate() {
        match parse_source(line, index + 1) {
            Ok(mut accounts) => imported.append(&mut accounts),
            Err(error) => return Err(format!("第 {} 行：{}", index + 1, error)),
        }
    }
    append_accounts(&state, imported)
}

fn import_json_bytes(bytes: &[u8], state: &AppState) -> Result<usize, String> {
    let backup: Backup = serde_json::from_slice(bytes).map_err(|e| format!("JSON 格式无效：{e}"))?;
    if backup.version != 1 { return Err(format!("不支持的备份版本：{}", backup.version)); }
    let incoming = backup.accounts.into_iter().map(Account::try_from).collect::<Result<Vec<_>, _>>()?;
    append_accounts(state, incoming)
}

#[tauri::command]
async fn import_json_dialog(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<usize, String> {
    let path = app.dialog().file().add_filter("JSON backup", &["json"]).blocking_pick_file();
    let Some(path) = path else { return Ok(0); };
    let path = path.into_path().map_err(|e| format!("文件路径无效：{e}"))?;
    let metadata = fs::metadata(&path).map_err(|e| format!("无法读取备份：{e}"))?;
    if metadata.len() > 10 * 1024 * 1024 { return Err("JSON 备份超过 10 MB 限制".into()); }
    let bytes = zeroize::Zeroizing::new(fs::read(path).map_err(|e| format!("无法读取备份：{e}"))?);
    import_json_bytes(&bytes, &state)
}

fn decode_qr(bytes: &[u8]) -> Result<Vec<String>, String> {
    if bytes.len() > 15 * 1024 * 1024 { return Err("图片超过 15 MB 限制".into()); }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8000);
    limits.max_image_height = Some(8000);
    limits.max_alloc = Some(96 * 1024 * 1024);
    let mut reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format().map_err(|e| format!("无法识别图片格式：{e}"))?;
    reader.limits(limits);
    let image = reader.decode().map_err(|e| format!("无法读取图片：{e}"))?.to_luma8();
    let mut prepared = PreparedImage::prepare(image);
    let grids = prepared.detect_grids();
    let mut payloads = Vec::new();
    for grid in grids {
        if let Ok((_, payload)) = grid.decode() { payloads.push(payload); }
    }
    if payloads.is_empty() { Err("图片中没有识别到二维码".into()) } else { Ok(payloads) }
}

fn import_qr_inner(bytes: Vec<u8>, state: &AppState) -> Result<usize, String> {
    let bytes = zeroize::Zeroizing::new(bytes);
    let mut incoming = Vec::new();
    for payload in decode_qr(&bytes)? {
        let payload = zeroize::Zeroizing::new(payload);
        if let Ok(mut accounts) = parse_source(payload.trim(), 1) { incoming.append(&mut accounts); }
    }
    append_accounts(state, incoming)
}

#[tauri::command]
fn import_qr(bytes: Vec<u8>, state: State<'_, AppState>) -> Result<usize, String> { import_qr_inner(bytes, &state) }

#[tauri::command]
fn try_import_qr(bytes: Vec<u8>, state: State<'_, AppState>) -> Result<usize, String> { import_qr_inner(bytes, &state) }

#[tauri::command]
fn delete_account(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut accounts = state.accounts.write().map_err(|_| lock_error())?;
    let index = accounts.iter().position(|account| account.id == id).ok_or("账号不存在")?;
    let removed = accounts.remove(index);
    if let Err(error) = persist(&state, &accounts) { accounts.insert(index, removed); return Err(error); }
    Ok(())
}

#[tauri::command]
async fn export_accounts(id: Option<String>, app: tauri::AppHandle, state: State<'_, AppState>) -> Result<bool, String> {
    let json = {
        let accounts = state.accounts.read().map_err(|_| lock_error())?;
        let selected: Vec<&Account> = match id.as_deref() {
            Some(id) => vec![accounts.iter().find(|account| account.id == id).ok_or("账号不存在")?],
            None => accounts.iter().collect(),
        };
        if selected.is_empty() { return Err("没有可导出的账号".into()); }
        let backup = Backup {
            format: "totp-desk".into(), version: 1,
            exported_at: SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_secs(),
            accounts: selected.into_iter().map(PersistedAccount::from).collect(),
        };
        zeroize::Zeroizing::new(serde_json::to_vec_pretty(&backup).map_err(|e| e.to_string())?)
    };
    let suffix = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_secs();
    let filename = if id.is_some() { format!("totp-desk-account-{suffix}.json") } else { format!("totp-desk-backup-{suffix}.json") };
    let path = app.dialog().file().add_filter("JSON backup", &["json"]).set_file_name(filename).blocking_save_file();
    let Some(path) = path else { return Ok(false); };
    let path = path.into_path().map_err(|e| format!("保存路径无效：{e}"))?;
    fs::write(path, json.as_slice()).map_err(|e| format!("写入备份失败：{e}"))?;
    Ok(true)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
            fs::create_dir_all(&dir)?;
            let path = dir.join("accounts.dat");
            let accounts = storage::load(&path).map_err(std::io::Error::other)?;
            app.manage(AppState { accounts: RwLock::new(accounts), path });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![list_accounts, add_account, update_account, import_text, import_json_dialog, import_qr, try_import_qr, delete_account, export_accounts])
        .run(tauri::generate_context!())
        .expect("failed to run TOTP Desk");
}

pub(crate) fn decode_migration(data: &str) -> Result<Vec<Account>, String> {
    let bytes = zeroize::Zeroizing::new(URL_SAFE_NO_PAD.decode(data.trim_end_matches('=').as_bytes()).map_err(|_| "Google 迁移数据编码无效")?);
    let payload = model::MigrationPayload::decode(bytes.as_slice()).map_err(|_| "Google 迁移数据无效")?;
    payload.into_accounts()
}
