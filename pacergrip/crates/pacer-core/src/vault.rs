use std::collections::HashMap;
use std::path::Path;

use aes::Aes128;
use cbc::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use uuid::Uuid;

type Aes128CbcEnc = cbc::Encryptor<Aes128>;
type Aes128CbcDec = cbc::Decryptor<Aes128>;

/// Legacy static AES-128 key used prior to v0.10.0. Retained solely to migrate existing
/// `credentials.dat` files transparently onto per-installation OS-protected `PCRV2` vaults.
const LEGACY_VAULT_KEY: [u8; 16] = [
    0x50, 0x61, 0x63, 0x65, 0x72, 0x47, 0x72, 0x69, 0x70, 0x56, 0x61, 0x75, 0x6C, 0x74, 0x32, 0x36,
];
const VAULT_V2_MAGIC: &[u8; 5] = b"PCRV2";
const VAULT_DATA_FILE: &str = "credentials.dat";
#[cfg(target_os = "windows")]
const VAULT_DPAPI_KEY_FILE: &str = "vault.dpapi";
const VAULT_PLAIN_KEY_FILE: &str = "vault.key";

#[cfg(target_os = "macos")]
const KEYCHAIN_SERVICE: &str = "cn.fwpt.pacer.vault";
#[cfg(target_os = "macos")]
const KEYCHAIN_ACCOUNT: &str = "master_key";

#[cfg(target_os = "windows")]
mod win_dpapi {
    #[repr(C)]
    struct CryptoApiBlob {
        cb_data: u32,
        pb_data: *mut u8,
    }

    #[link(name = "crypt32")]
    extern "system" {
        fn CryptProtectData(
            p_data_in: *const CryptoApiBlob,
            sz_data_descr: *const u16,
            p_optional_entropy: *const CryptoApiBlob,
            pv_reserved: *mut core::ffi::c_void,
            p_prompt_struct: *mut core::ffi::c_void,
            dw_flags: u32,
            p_data_out: *mut CryptoApiBlob,
        ) -> i32;

        fn CryptUnprotectData(
            p_data_in: *const CryptoApiBlob,
            ppsz_data_descr: *mut *mut u16,
            p_optional_entropy: *const CryptoApiBlob,
            pv_reserved: *mut core::ffi::c_void,
            p_prompt_struct: *mut core::ffi::c_void,
            dw_flags: u32,
            p_data_out: *mut CryptoApiBlob,
        ) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn LocalFree(hmem: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
    }

    const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x1;

    pub fn protect(plain: &[u8]) -> Option<Vec<u8>> {
        let input = CryptoApiBlob {
            cb_data: u32::try_from(plain.len()).ok()?,
            pb_data: plain.as_ptr() as *mut u8,
        };
        let mut output = CryptoApiBlob {
            cb_data: 0,
            pb_data: core::ptr::null_mut(),
        };
        // SAFETY: `input` points to valid `plain` bytes for the synchronous FFI call.
        let ok = unsafe {
            CryptProtectData(
                &input,
                core::ptr::null(),
                core::ptr::null(),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if ok == 0 || output.pb_data.is_null() {
            return None;
        }
        // SAFETY: `CryptProtectData` succeeded and returned an allocated buffer freed via `LocalFree`.
        let out = unsafe {
            let bytes = core::slice::from_raw_parts(output.pb_data, output.cb_data as usize).to_vec();
            let _ = LocalFree(output.pb_data as *mut core::ffi::c_void);
            bytes
        };
        Some(out)
    }

    pub fn unprotect(cipher: &[u8]) -> Option<Vec<u8>> {
        let input = CryptoApiBlob {
            cb_data: u32::try_from(cipher.len()).ok()?,
            pb_data: cipher.as_ptr() as *mut u8,
        };
        let mut output = CryptoApiBlob {
            cb_data: 0,
            pb_data: core::ptr::null_mut(),
        };
        // SAFETY: `input` points to valid `cipher` bytes; descriptor pointer is null.
        let ok = unsafe {
            CryptUnprotectData(
                &input,
                core::ptr::null_mut(),
                core::ptr::null(),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if ok == 0 || output.pb_data.is_null() {
            return None;
        }
        // SAFETY: `CryptUnprotectData` succeeded and returned an allocated buffer freed via `LocalFree`.
        let out = unsafe {
            let bytes = core::slice::from_raw_parts(output.pb_data, output.cb_data as usize).to_vec();
            let _ = LocalFree(output.pb_data as *mut core::ffi::c_void);
            bytes
        };
        Some(out)
    }
}

#[cfg(target_os = "macos")]
fn load_keychain_master_key() -> Option<[u8; 16]> {
    let out = std::process::Command::new("/usr/bin/security")
        .args([
            "find-generic-password",
            "-a",
            KEYCHAIN_ACCOUNT,
            "-s",
            KEYCHAIN_SERVICE,
            "-w",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    decode_hex_16(String::from_utf8_lossy(&out.stdout).trim())
}

#[cfg(target_os = "macos")]
fn store_keychain_master_key(key: &[u8; 16]) -> bool {
    let hex = encode_hex(key);
    std::process::Command::new("/usr/bin/security")
        .args([
            "add-generic-password",
            "-a",
            KEYCHAIN_ACCOUNT,
            "-s",
            KEYCHAIN_SERVICE,
            "-w",
            &hex,
            "-U",
        ])
        .status()
        .is_ok_and(|s| s.success())
}

#[cfg(target_os = "macos")]
fn encode_hex(bytes: &[u8; 16]) -> String {
    use std::fmt::Write as _;

    let mut s = String::with_capacity(32);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

#[cfg(target_os = "macos")]
fn decode_hex_16(raw: &str) -> Option<[u8; 16]> {
    if raw.len() != 32 {
        return None;
    }
    let mut out = [0u8; 16];
    for (idx, chunk) in raw.as_bytes().chunks_exact(2).enumerate() {
        let hex_pair = core::str::from_utf8(chunk).ok()?;
        out[idx] = u8::from_str_radix(hex_pair, 16).ok()?;
    }
    Some(out)
}

#[cfg(target_os = "windows")]
fn try_load_os_master_key(storage_dir: &Path) -> Option<[u8; 16]> {
    let blob = std::fs::read(storage_dir.join(VAULT_DPAPI_KEY_FILE)).ok()?;
    let plain = win_dpapi::unprotect(&blob)?;
    <[u8; 16]>::try_from(plain.as_slice()).ok()
}

#[cfg(target_os = "macos")]
fn try_load_os_master_key(_storage_dir: &Path) -> Option<[u8; 16]> {
    load_keychain_master_key()
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn try_load_os_master_key(_storage_dir: &Path) -> Option<[u8; 16]> {
    None
}

#[cfg(target_os = "windows")]
fn try_store_os_master_key(storage_dir: &Path, key: &[u8; 16]) -> Result<bool, String> {
    if let Some(protected) = win_dpapi::protect(key) {
        std::fs::write(storage_dir.join(VAULT_DPAPI_KEY_FILE), protected)
            .map_err(|e| e.to_string())?;
        return Ok(true);
    }
    Ok(false)
}

#[cfg(target_os = "macos")]
fn try_store_os_master_key(_storage_dir: &Path, key: &[u8; 16]) -> Result<bool, String> {
    Ok(store_keychain_master_key(key))
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn try_store_os_master_key(_storage_dir: &Path, _key: &[u8; 16]) -> Result<bool, String> {
    Ok(false)
}

fn write_restricted_key_file(path: &Path, key: &[u8; 16]) -> Result<(), String> {
    std::fs::write(path, key).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

fn try_load_existing_master_key(storage_dir: &Path, is_portable: bool) -> Option<[u8; 16]> {
    if !is_portable {
        let os_key = try_load_os_master_key(storage_dir);
        if os_key.is_some() {
            return os_key;
        }
    }

    let raw = std::fs::read(storage_dir.join(VAULT_PLAIN_KEY_FILE)).ok()?;
    <[u8; 16]>::try_from(raw.as_slice()).ok()
}

fn load_or_create_master_key(storage_dir: &Path, is_portable: bool) -> Result<[u8; 16], String> {
    if let Some(existing) = try_load_existing_master_key(storage_dir, is_portable) {
        return Ok(existing);
    }

    std::fs::create_dir_all(storage_dir).map_err(|e| e.to_string())?;
    let new_key = *Uuid::new_v4().as_bytes();

    if !is_portable && try_store_os_master_key(storage_dir, &new_key)? {
        return Ok(new_key);
    }

    write_restricted_key_file(&storage_dir.join(VAULT_PLAIN_KEY_FILE), &new_key)?;
    Ok(new_key)
}

fn decrypt_vault_payload(key: &[u8; 16], iv_and_ct: &[u8]) -> Option<HashMap<String, String>> {
    if iv_and_ct.len() <= 16 {
        return None;
    }
    let (iv, ct) = iv_and_ct.split_at(16);
    let mut buf = ct.to_vec();
    let dec = Aes128CbcDec::new_from_slices(key, iv).ok()?;
    let pt = dec.decrypt_padded_mut::<Pkcs7>(&mut buf).ok()?;
    serde_json::from_slice::<HashMap<String, String>>(pt).ok()
}

/// Loads persisted connection passwords from `storage_dir/credentials.dat`.
/// Automatically migrates pre-`0.10.0` legacy vaults into OS-protected `PCRV2` format.
pub fn load_vault_passwords(storage_dir: &Path, is_portable: bool) -> HashMap<String, String> {
    let path = storage_dir.join(VAULT_DATA_FILE);
    let Ok(raw) = std::fs::read(&path) else {
        return HashMap::new();
    };

    if let Some(rest) = raw.strip_prefix(VAULT_V2_MAGIC) {
        let Some(master_key) = try_load_existing_master_key(storage_dir, is_portable) else {
            return HashMap::new();
        };
        return decrypt_vault_payload(&master_key, rest).unwrap_or_default();
    }

    // Legacy v1 compatibility path -> upgrade automatically to PCRV2 on success
    let Some(legacy_map) = decrypt_vault_payload(&LEGACY_VAULT_KEY, &raw) else {
        return HashMap::new();
    };
    if !legacy_map.is_empty() {
        let _ = save_vault_passwords(storage_dir, is_portable, &legacy_map);
    }
    legacy_map
}

/// Encrypts and writes persisted connection passwords to `storage_dir/credentials.dat` using a
/// per-installation random 128-bit master key protected by Windows DPAPI / macOS Keychain when
/// running in standard desktop mode, or `vault.key` in portable mode.
pub fn save_vault_passwords(
    storage_dir: &Path,
    is_portable: bool,
    map: &HashMap<String, String>,
) -> Result<(), String> {
    std::fs::create_dir_all(storage_dir).map_err(|e| e.to_string())?;
    let path = storage_dir.join(VAULT_DATA_FILE);
    if map.is_empty() {
        let _ = std::fs::remove_file(path);
        return Ok(());
    }

    let master_key = load_or_create_master_key(storage_dir, is_portable)?;
    let pt = serde_json::to_vec(map).map_err(|e| e.to_string())?;
    let iv = *Uuid::new_v4().as_bytes();
    let mut buf = vec![0u8; pt.len() + 16];
    buf[..pt.len()].copy_from_slice(&pt);
    let enc = Aes128CbcEnc::new_from_slices(&master_key, &iv).map_err(|e| e.to_string())?;
    let ct = enc
        .encrypt_padded_mut::<Pkcs7>(&mut buf, pt.len())
        .map_err(|_| "vault encrypt error".to_string())?;

    let mut out = Vec::with_capacity(VAULT_V2_MAGIC.len() + 16 + ct.len());
    out.extend_from_slice(VAULT_V2_MAGIC);
    out.extend_from_slice(&iv);
    out.extend_from_slice(ct);
    std::fs::write(path, out).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pcrv2_vault_roundtrip_standard_and_portable() {
        for portable in [false, true] {
            let dir = std::env::temp_dir().join(format!(
                "pacer_vault_rt_{}_{}",
                portable,
                Uuid::new_v4()
            ));
            let _ = std::fs::remove_dir_all(&dir);

            let mut secrets = HashMap::new();
            secrets.insert("prof-1".to_string(), "P@ssw0rd!#1".to_string());
            secrets.insert("ssh_pwd:prof-1".to_string(), "SshSecret".to_string());

            save_vault_passwords(&dir, portable, &secrets).unwrap();
            let raw = std::fs::read(dir.join(VAULT_DATA_FILE)).unwrap();
            assert!(raw.starts_with(VAULT_V2_MAGIC));

            let loaded = load_vault_passwords(&dir, portable);
            assert_eq!(loaded, secrets);

            // Clearing secrets removes credentials.dat cleanly
            save_vault_passwords(&dir, portable, &HashMap::new()).unwrap();
            assert!(!dir.join(VAULT_DATA_FILE).exists());
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn test_legacy_v1_vault_auto_migrates_to_pcrv2() {
        let dir = std::env::temp_dir().join(format!("pacer_vault_mig_{}", Uuid::new_v4()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut secrets = HashMap::new();
        secrets.insert("legacy-conn".to_string(), "old_secret_value".to_string());
        let pt = serde_json::to_vec(&secrets).unwrap();
        let iv = *Uuid::new_v4().as_bytes();
        let mut buf = vec![0u8; pt.len() + 16];
        buf[..pt.len()].copy_from_slice(&pt);
        let enc = Aes128CbcEnc::new_from_slices(&LEGACY_VAULT_KEY, &iv).unwrap();
        let ct = enc.encrypt_padded_mut::<Pkcs7>(&mut buf, pt.len()).unwrap();
        let mut legacy_bytes = Vec::with_capacity(16 + ct.len());
        legacy_bytes.extend_from_slice(&iv);
        legacy_bytes.extend_from_slice(ct);
        std::fs::write(dir.join(VAULT_DATA_FILE), legacy_bytes).unwrap();

        let loaded = load_vault_passwords(&dir, true);
        assert_eq!(loaded.get("legacy-conn").map(String::as_str), Some("old_secret_value"));

        let migrated_raw = std::fs::read(dir.join(VAULT_DATA_FILE)).unwrap();
        assert!(migrated_raw.starts_with(VAULT_V2_MAGIC));
        assert_eq!(load_vault_passwords(&dir, true), secrets);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
