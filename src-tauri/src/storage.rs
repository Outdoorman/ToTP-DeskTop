use std::{fs, io::Write, path::Path};
use zeroize::Zeroizing;

use crate::{crypto, model::{Account, Backup, PersistedAccount}};

pub fn save(path: &Path, accounts: &[Account]) -> Result<(), String> {
    let backup = Backup { format: "totp-desk-internal".into(), version: 1, exported_at: 0, accounts: accounts.iter().map(PersistedAccount::from).collect() };
    let plain = Zeroizing::new(serde_json::to_vec(&backup).map_err(|e| format!("序列化失败：{e}"))?);
    let cipher = crypto::protect(&plain)?;
    let temp = path.with_extension("tmp");
    let backup_path = path.with_extension("bak");
    let mut file = fs::File::create(&temp).map_err(|e| format!("创建本地数据失败：{e}"))?;
    file.write_all(&cipher).and_then(|_| file.sync_all()).map_err(|e| format!("保存本地数据失败：{e}"))?;
    drop(file);
    if path.exists() {
        fs::copy(path, &backup_path).map_err(|e| format!("创建安全副本失败：{e}"))?;
        fs::remove_file(path).map_err(|e| format!("替换本地数据失败：{e}"))?;
    }
    if let Err(error) = fs::rename(&temp, path) {
        if backup_path.exists() { let _ = fs::copy(&backup_path, path); }
        return Err(format!("提交本地数据失败：{error}"));
    }
    if backup_path.exists() { let _ = fs::remove_file(backup_path); }
    Ok(())
}

pub fn load(path: &Path) -> Result<Vec<Account>, String> {
    let backup_path = path.with_extension("bak");
    let source = if path.exists() { path } else if backup_path.exists() { backup_path.as_path() } else { return Ok(Vec::new()); };
    let cipher = fs::read(source).map_err(|e| format!("读取本地数据失败：{e}"))?;
    let plain = crypto::unprotect(&cipher)?;
    let backup: Backup = serde_json::from_slice(&plain).map_err(|e| format!("本地数据损坏：{e}"))?;
    backup.accounts.into_iter().map(Account::try_from).collect()
}
