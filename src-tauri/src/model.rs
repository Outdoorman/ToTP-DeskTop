use data_encoding::BASE32_NOPAD;
use prost::Message;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

use crate::{decode_migration, otp};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Algorithm { Sha1, Sha256, Sha512 }

impl Algorithm {
    fn parse(value: &str) -> Result<Self, String> {
        match value.to_ascii_uppercase().as_str() { "SHA1" => Ok(Self::Sha1), "SHA256" => Ok(Self::Sha256), "SHA512" => Ok(Self::Sha512), _ => Err("仅支持 SHA1、SHA256 或 SHA512".into()) }
    }
}

pub struct Account {
    pub id: String,
    pub label: String,
    pub issuer: String,
    pub secret: Zeroizing<Vec<u8>>,
    pub algorithm: Algorithm,
    pub digits: u32,
    pub period: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInput { pub secret: String, pub label: String, pub issuer: String, pub algorithm: Algorithm, pub digits: u32, pub period: u64 }

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountUpdateInput { pub id: String, pub secret: String, pub label: String, pub issuer: String, pub algorithm: Algorithm, pub digits: u32, pub period: u64 }

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountView { id: String, label: String, issuer: String, algorithm: Algorithm, digits: u32, period: u64, code: String, remaining: u64 }

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedAccount { pub id: String, pub label: String, pub issuer: String, pub secret: String, pub algorithm: Algorithm, pub digits: u32, pub period: u64 }

impl Drop for PersistedAccount { fn drop(&mut self) { self.secret.zeroize(); } }

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup { pub format: String, pub version: u32, #[serde(default)] pub exported_at: u64, pub accounts: Vec<PersistedAccount> }

fn decode_secret(secret: &str) -> Result<Zeroizing<Vec<u8>>, String> {
    let normalized: Zeroizing<String> = Zeroizing::new(secret.chars().filter(|c| !c.is_ascii_whitespace() && *c != '-' && *c != '=').flat_map(char::to_uppercase).collect());
    if normalized.len() < 16 { return Err("Seed 至少需要 16 个 Base32 字符".into()) }
    BASE32_NOPAD.decode(normalized.as_bytes()).map(Zeroizing::new).map_err(|_| "Seed 不是有效的 Base32 编码".into())
}

fn validate(digits: u32, period: u64) -> Result<(), String> {
    if !matches!(digits, 6 | 8) { return Err("验证码位数只能是 6 或 8".into()); }
    if !(15..=120).contains(&period) { return Err("周期必须在 15–120 秒之间".into()); }
    Ok(())
}

impl TryFrom<AccountInput> for Account {
    type Error = String;
    fn try_from(mut value: AccountInput) -> Result<Self, Self::Error> {
        validate(value.digits, value.period)?;
        let secret_text = Zeroizing::new(std::mem::take(&mut value.secret));
        let secret = decode_secret(&secret_text)?;
        Ok(Self { id: Uuid::new_v4().to_string(), label: value.label.trim().to_string(), issuer: value.issuer.trim().to_string(), secret, algorithm: value.algorithm, digits: value.digits, period: value.period })
    }
}

impl TryFrom<PersistedAccount> for Account {
    type Error = String;
    fn try_from(value: PersistedAccount) -> Result<Self, Self::Error> {
        validate(value.digits, value.period)?;
        let secret = decode_secret(&value.secret)?;
        Ok(Self { id: if value.id.is_empty() { Uuid::new_v4().to_string() } else { value.id.clone() }, label: value.label.clone(), issuer: value.issuer.clone(), secret, algorithm: value.algorithm, digits: value.digits, period: value.period })
    }
}

impl From<&Account> for PersistedAccount {
    fn from(value: &Account) -> Self { Self { id: value.id.clone(), label: value.label.clone(), issuer: value.issuer.clone(), secret: BASE32_NOPAD.encode(&value.secret), algorithm: value.algorithm, digits: value.digits, period: value.period } }
}

impl Account {
    pub fn updated(&self, mut value: AccountUpdateInput) -> Result<Self, String> {
        validate(value.digits, value.period)?;
        let secret_text = Zeroizing::new(std::mem::take(&mut value.secret));
        let secret = if secret_text.trim().is_empty() { self.secret.clone() } else { decode_secret(&secret_text)? };
        Ok(Self {
            id: self.id.clone(),
            label: value.label.trim().to_string(),
            issuer: value.issuer.trim().to_string(),
            secret,
            algorithm: value.algorithm,
            digits: value.digits,
            period: value.period,
        })
    }

    pub fn view(&self, now: u64) -> Result<AccountView, String> {
        Ok(AccountView { id: self.id.clone(), label: self.label.clone(), issuer: self.issuer.clone(), algorithm: self.algorithm, digits: self.digits, period: self.period, code: otp::generate(&self.secret, self.algorithm, self.digits, self.period, now)?, remaining: self.period - now % self.period })
    }
}

#[cfg(test)]
mod account_tests {
    use super::*;

    #[test]
    fn update_keeps_existing_secret_when_seed_is_blank() {
        let account = Account::try_from(AccountInput { secret: "JBSWY3DPEHPK3PXP".into(), label: "old".into(), issuer: "Demo".into(), algorithm: Algorithm::Sha1, digits: 6, period: 30 }).unwrap();
        let updated = account.updated(AccountUpdateInput { id: account.id.clone(), secret: String::new(), label: "new".into(), issuer: "Demo 2".into(), algorithm: Algorithm::Sha256, digits: 8, period: 60 }).unwrap();
        assert_eq!(updated.secret.as_slice(), account.secret.as_slice());
        assert_eq!(updated.label, "new");
        assert_eq!(updated.digits, 8);
        assert_eq!(updated.period, 60);
    }
}

fn parse_otpauth(source: &str) -> Result<Account, String> {
    let url = Url::parse(source).map_err(|_| "otpauth 链接无效")?;
    if url.scheme() != "otpauth" || url.host_str() != Some("totp") { return Err("仅支持 otpauth://totp 链接".into()); }
    let mut secret = None; let mut issuer = String::new(); let mut algorithm = Algorithm::Sha1; let mut digits = 6; let mut period = 30;
    for (key, value) in url.query_pairs() {
        match key.as_ref() { "secret" => secret = Some(value.into_owned()), "issuer" => issuer = value.into_owned(), "algorithm" => algorithm = Algorithm::parse(&value)?, "digits" => digits = value.parse().map_err(|_| "digits 参数无效")?, "period" => period = value.parse().map_err(|_| "period 参数无效")?, _ => {} }
    }
    validate(digits, period)?;
    let path = url.path().trim_start_matches('/');
    let decoded = percent_decode(path);
    let label = if let Some((prefix, account)) = decoded.split_once(':') { if issuer.is_empty() { issuer = prefix.to_string(); } account.to_string() } else { decoded };
    let secret = Zeroizing::new(secret.ok_or("otpauth 链接缺少 secret 参数")?);
    let bytes = decode_secret(&secret)?;
    Ok(Account { id: Uuid::new_v4().to_string(), label, issuer, secret: bytes, algorithm, digits, period })
}

fn percent_decode(value: &str) -> String {
    url::form_urlencoded::parse(format!("v={value}").as_bytes()).find(|(k, _)| k == "v").map(|(_, v)| v.into_owned()).unwrap_or_default()
}

pub fn parse_source(source: &str, index: usize) -> Result<Vec<Account>, String> {
    if source.starts_with("otpauth://") { Ok(vec![parse_otpauth(source)?]) }
    else if source.starts_with("otpauth-migration://") {
        let url = Url::parse(source).map_err(|_| "Google 迁移链接无效")?;
        let data = Zeroizing::new(url.query_pairs().find(|(k, _)| k == "data").map(|(_, v)| v.into_owned()).ok_or("Google 迁移链接缺少 data")?);
        decode_migration(&data)
    } else {
        let secret = decode_secret(source)?;
        Ok(vec![Account { id: Uuid::new_v4().to_string(), label: format!("导入账号 {index}"), issuer: "TOTP".into(), secret, algorithm: Algorithm::Sha1, digits: 6, period: 30 }])
    }
}

#[derive(Clone, PartialEq, Message)]
pub struct MigrationPayload {
    #[prost(message, repeated, tag="1")] pub otp_parameters: Vec<OtpParameters>,
    #[prost(int32, tag="2")] pub version: i32,
    #[prost(int32, tag="3")] pub batch_size: i32,
    #[prost(int32, tag="4")] pub batch_index: i32,
    #[prost(int32, tag="5")] pub batch_id: i32,
}

#[derive(Clone, PartialEq, Message)]
pub struct OtpParameters {
    #[prost(bytes="vec", tag="1")] pub secret: Vec<u8>,
    #[prost(string, tag="2")] pub name: String,
    #[prost(string, tag="3")] pub issuer: String,
    #[prost(int32, tag="4")] pub algorithm: i32,
    #[prost(int32, tag="5")] pub digits: i32,
    #[prost(int32, tag="6")] pub otp_type: i32,
    #[prost(int64, tag="7")] pub counter: i64,
}

impl MigrationPayload {
    pub fn into_accounts(self) -> Result<Vec<Account>, String> {
        let mut accounts = Vec::new();
        for mut item in self.otp_parameters {
            if item.otp_type == 1 || item.secret.is_empty() { item.secret.zeroize(); continue; }
            let algorithm = match item.algorithm { 0 | 1 => Algorithm::Sha1, 2 => Algorithm::Sha256, 3 => Algorithm::Sha512, _ => { item.secret.zeroize(); continue; } };
            let digits = if item.digits == 2 { 8 } else { 6 };
            let label = item.name.strip_prefix(&format!("{}:", item.issuer)).unwrap_or(&item.name).to_string();
            accounts.push(Account { id: Uuid::new_v4().to_string(), label, issuer: item.issuer, secret: Zeroizing::new(std::mem::take(&mut item.secret)), algorithm, digits, period: 30 });
        }
        if accounts.is_empty() { Err("迁移码中没有可用的 TOTP 账号".into()) } else { Ok(accounts) }
    }
}
