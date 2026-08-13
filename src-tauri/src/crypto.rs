use zeroize::Zeroizing;

#[cfg(windows)]
pub fn protect(plain: &[u8]) -> Result<Vec<u8>, String> {
    use std::{ptr, slice};
    use windows_sys::Win32::{Foundation::LocalFree, Security::Cryptography::{CryptProtectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN}};
    let input = CRYPT_INTEGER_BLOB { cbData: plain.len() as u32, pbData: plain.as_ptr() as *mut u8 };
    let mut output = CRYPT_INTEGER_BLOB { cbData: 0, pbData: ptr::null_mut() };
    let ok = unsafe { CryptProtectData(&input, ptr::null(), ptr::null(), ptr::null(), ptr::null(), CRYPTPROTECT_UI_FORBIDDEN, &mut output) };
    if ok == 0 { return Err("Windows DPAPI 加密失败".into()); }
    let result = unsafe { slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe { LocalFree(output.pbData.cast()); }
    Ok(result)
}

#[cfg(windows)]
pub fn unprotect(cipher: &[u8]) -> Result<Zeroizing<Vec<u8>>, String> {
    use std::{ptr, slice};
    use windows_sys::Win32::{Foundation::LocalFree, Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN}};
    let input = CRYPT_INTEGER_BLOB { cbData: cipher.len() as u32, pbData: cipher.as_ptr() as *mut u8 };
    let mut output = CRYPT_INTEGER_BLOB { cbData: 0, pbData: ptr::null_mut() };
    let ok = unsafe { CryptUnprotectData(&input, ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), CRYPTPROTECT_UI_FORBIDDEN, &mut output) };
    if ok == 0 { return Err("无法解密本地数据（可能来自其他 Windows 用户）".into()); }
    let result = Zeroizing::new(unsafe { slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() });
    unsafe {
        ptr::write_bytes(output.pbData, 0, output.cbData as usize);
        LocalFree(output.pbData.cast());
    }
    Ok(result)
}

#[cfg(not(windows))]
pub fn protect(_: &[u8]) -> Result<Vec<u8>, String> { Err("此版本只支持 Windows DPAPI".into()) }
#[cfg(not(windows))]
pub fn unprotect(_: &[u8]) -> Result<Zeroizing<Vec<u8>>, String> { Err("此版本只支持 Windows DPAPI".into()) }
