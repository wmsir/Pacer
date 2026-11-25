use std::collections::HashMap;
use std::path::Path;

use aes::{Aes128, Aes256};
use cbc::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use uuid::Uuid;
use zeroize::Zeroize;

type Aes128CbcDec = cbc::Decryptor<Aes128>;
type Aes256CbcEnc = cbc::Encryptor<Aes256>;
type Aes256CbcDec = cbc::Decryptor<Aes256>;
type HmacSha256 = Hmac<Sha256>;

/// Legacy static AES-128 key used prior to v0.10.0. Retained solely to migrate existing
/// `credentials.dat` files transparently onto authenticated OS-protected `PCRV3` vaults.
const LEGACY_VAULT_KEY: [u8; 16] = [
    0x50, 0x61, 0x63, 0x65, 0x72, 0x47, 0x72, 0x69, 0x70, 0x56, 0x61, 0x75, 0x6C, 0x74, 0x32, 0x36,
];
const VAULT_V2_MAGIC: &[u8; 5] = b"PCRV2";
const VAULT_V3_MAGIC: &[u8; 5] = b"PCRV3";
const VAULT_DATA_FILE: &str = "credentials.dat";
#[cfg(target_os = "windows")]
const VAULT_DPAPI_KEY_FILE: &str = "vault.dpapi";
#[cfg(target_os = "windows")]
const VAULT_DPAPI_V3_KEY_FILE: &str = "vault_v3.dpapi";
const VAULT_PLAIN_KEY_FILE: &str = "vault.key";
const VAULT_PLAIN_V3_KEY_FILE: &str = "vault_v3.key";

#[cfg(target_os = "macos")]
const KEYCHAIN_SERVICE: &str = "cn.fwpt.pacer.vault";
#[cfg(target_os = "macos")]
const KEYCHAIN_V3_SERVICE: &str = "cn.fwpt.pacer.vault.v3";
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
fn load_keychain_key(service: &str) -> Option<Vec<u8>> {
    let out = std::process::Command::new("/usr/bin/security")
        .args([
            "find-generic-password",
            "-a",
            KEYCHAIN_ACCOUNT,
            "-s",
            service,
            "-w",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    decode_hex_bytes(String::from_utf8_lossy(&out.stdout).trim())
}

#[cfg(target_os = "macos")]
fn store_keychain_v3_master_key(key: &[u8; 32]) -> bool {
    use std::io::Write as _;
    use std::process::{Command, Stdio};

    let mut hex = encode_hex_bytes(key);
    let mut cmd_line = format!(
        "add-generic-password -a {KEYCHAIN_ACCOUNT} -s {KEYCHAIN_V3_SERVICE} -w \"{hex}\" -U\n"
    );
    hex.zeroize();

    let Ok(mut child) = Command::new("/usr/bin/security")
        .arg("-i")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        cmd_line.zeroize();
        return false;
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(cmd_line.as_bytes());
    }
    cmd_line.zeroize();
    child.wait().is_ok_and(|s| s.success())
}

#[cfg(target_os = "macos")]
fn encode_hex_bytes(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

#[cfg(target_os = "macos")]
fn decode_hex_bytes(raw: &str) -> Option<Vec<u8>> {
    let (chunks, rem) = raw.as_bytes().as_chunks::<2>();
    if !rem.is_empty() || chunks.is_empty() {
        return None;
    }
    let mut out = Vec::with_capacity(chunks.len());
    for pair in chunks {
        let hex_pair = core::str::from_utf8(pair).ok()?;
        out.push(u8::from_str_radix(hex_pair, 16).ok()?);
    }
    Some(out)
}

#[cfg(target_os = "windows")]
fn try_load_os_master_key_v2(storage_dir: &Path) -> Option<[u8; 16]> {
    let blob = std::fs::read(storage_dir.join(VAULT_DPAPI_KEY_FILE)).ok()?;
    let mut plain = win_dpapi::unprotect(&blob)?;
    let arr = <[u8; 16]>::try_from(plain.as_slice()).ok();
    plain.zeroize();
    arr
}

#[cfg(target_os = "macos")]
fn try_load_os_master_key_v2(_storage_dir: &Path) -> Option<[u8; 16]> {
    let mut raw = load_keychain_key(KEYCHAIN_SERVICE)?;
    let arr = <[u8; 16]>::try_from(raw.as_slice()).ok();
    raw.zeroize();
    arr
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn try_load_os_master_key_v2(_storage_dir: &Path) -> Option<[u8; 16]> {
    None
}

#[cfg(target_os = "windows")]
fn try_load_os_master_key_v3(storage_dir: &Path) -> Option<[u8; 32]> {
    let blob = std::fs::read(storage_dir.join(VAULT_DPAPI_V3_KEY_FILE)).ok()?;
    let mut plain = win_dpapi::unprotect(&blob)?;
    let arr = <[u8; 32]>::try_from(plain.as_slice()).ok();
    plain.zeroize();
    arr
}

#[cfg(target_os = "macos")]
fn try_load_os_master_key_v3(_storage_dir: &Path) -> Option<[u8; 32]> {
    let mut raw = load_keychain_key(KEYCHAIN_V3_SERVICE)?;
    let arr = <[u8; 32]>::try_from(raw.as_slice()).ok();
    raw.zeroize();
    arr
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn try_load_os_master_key_v3(_storage_dir: &Path) -> Option<[u8; 32]> {
    None
}

#[cfg(target_os = "windows")]
fn try_store_os_master_key_v3(storage_dir: &Path, key: &[u8; 32]) -> Result<bool, String> {
    if let Some(protected) = win_dpapi::protect(key) {
        write_atomic_restricted_file(&storage_dir.join(VAULT_DPAPI_V3_KEY_FILE), &protected)?;
        return Ok(true);
    }
    Ok(false)
}

#[cfg(target_os = "macos")]
fn try_store_os_master_key_v3(_storage_dir: &Path, key: &[u8; 32]) -> Result<bool, String> {
    Ok(store_keychain_v3_master_key(key))
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn try_store_os_master_key_v3(_storage_dir: &Path, _key: &[u8; 32]) -> Result<bool, String> {
    Ok(false)
}

fn write_atomic_restricted_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("vault");
    let tmp_path = path.with_file_name(format!(".{file_name}.{}.tmp", Uuid::new_v4()));
    std::fs::write(&tmp_path, bytes).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o600));
    }
    if let Err(err) = std::fs::rename(&tmp_path, path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(err.to_string());
    }
    Ok(())
}

fn try_load_existing_master_key_v2(storage_dir: &Path, is_portable: bool) -> Option<[u8; 16]> {
    if !is_portable {
        let os_key = try_load_os_master_key_v2(storage_dir);
        if os_key.is_some() {
            return os_key;
        }
    }

    let mut raw = std::fs::read(storage_dir.join(VAULT_PLAIN_KEY_FILE)).ok()?;
    let arr = <[u8; 16]>::try_from(raw.as_slice()).ok();
    raw.zeroize();
    arr
}

fn try_load_existing_master_key_v3(storage_dir: &Path, is_portable: bool) -> Option<[u8; 32]> {
    if !is_portable {
        let os_key = try_load_os_master_key_v3(storage_dir);
        if os_key.is_some() {
            return os_key;
        }
    }

    let mut raw = std::fs::read(storage_dir.join(VAULT_PLAIN_V3_KEY_FILE)).ok()?;
    let arr = <[u8; 32]>::try_from(raw.as_slice()).ok();
    raw.zeroize();
    arr
}

fn generate_256_bit_master_key() -> [u8; 32] {
    let mut hasher = Sha256::new();
    for _ in 0..3 {
        let u = Uuid::new_v4();
        hasher.update(u.as_bytes());
    }
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

fn load_or_create_master_key_v3(storage_dir: &Path, is_portable: bool) -> Result<[u8; 32], String> {
    if let Some(existing) = try_load_existing_master_key_v3(storage_dir, is_portable) {
        return Ok(existing);
    }

    std::fs::create_dir_all(storage_dir).map_err(|e| e.to_string())?;
    let new_key = generate_256_bit_master_key();

    if !is_portable && try_store_os_master_key_v3(storage_dir, &new_key)? {
        return Ok(new_key);
    }

    write_atomic_restricted_file(&storage_dir.join(VAULT_PLAIN_V3_KEY_FILE), &new_key)?;
    Ok(new_key)
}

fn derive_v3_subkeys(master_key: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
    let mut enc_hash = Sha256::new();
    enc_hash.update(b"pacer-vault-enc-v3");
    enc_hash.update(master_key);
    let mut enc_key = [0u8; 32];
    enc_key.copy_from_slice(&enc_hash.finalize());

    let mut mac_hash = Sha256::new();
    mac_hash.update(b"pacer-vault-mac-v3");
    mac_hash.update(master_key);
    let mut mac_key = [0u8; 32];
    mac_key.copy_from_slice(&mac_hash.finalize());

    (enc_key, mac_key)
}

fn decrypt_vault_payload_v1_or_v2(
    key: &[u8; 16],
    iv_and_ct: &[u8],
) -> Option<HashMap<String, String>> {
    if iv_and_ct.len() <= 16 {
        return None;
    }
    let (iv, ct) = iv_and_ct.split_at(16);
    let mut buf = ct.to_vec();
    let dec = Aes128CbcDec::new_from_slices(key, iv).ok()?;
    let pt = dec.decrypt_padded_mut::<Pkcs7>(&mut buf).ok()?;
    let parsed = serde_json::from_slice::<HashMap<String, String>>(pt).ok();
    buf.zeroize();
    parsed
}

fn decrypt_vault_payload_v3(
    master_key: &[u8; 32],
    body: &[u8],
) -> Option<HashMap<String, String>> {
    // body layout: [16B IV][ciphertext (>=16B)][32B HMAC-SHA256 tag]
    if body.len() <= 16 + 32 {
        return None;
    }
    let tag_offset = body.len() - 32;
    let (iv_and_ct, tag) = body.split_at(tag_offset);
    let (mut enc_key, mut mac_key) = derive_v3_subkeys(master_key);

    let mut mac = <HmacSha256 as Mac>::new_from_slice(&mac_key).ok()?;
    mac.update(VAULT_V3_MAGIC);
    mac.update(iv_and_ct);
    let expected = mac.finalize().into_bytes();
    let tag_valid = bool::from(expected.as_slice().ct_eq(tag));
    mac_key.zeroize();
    if !tag_valid {
        enc_key.zeroize();
        return None;
    }

    let (iv, ct) = iv_and_ct.split_at(16);
    let mut buf = ct.to_vec();
    let dec = Aes256CbcDec::new_from_slices(&enc_key, iv).ok();
    enc_key.zeroize();
    let dec = dec?;
    let parsed = match dec.decrypt_padded_mut::<Pkcs7>(&mut buf) {
        Ok(pt) => serde_json::from_slice::<HashMap<String, String>>(pt).ok(),
        Err(_) => None,
    };
    buf.zeroize();
    parsed
}

/// Loads persisted connection passwords from `storage_dir/credentials.dat`.
/// Automatically migrates pre-`0.10.0` legacy (`v1`) and `PCRV2` vaults onto authenticated `PCRV3`.
pub fn load_vault_passwords(storage_dir: &Path, is_portable: bool) -> HashMap<String, String> {
    let path = storage_dir.join(VAULT_DATA_FILE);
    let Ok(raw) = std::fs::read(&path) else {
        return HashMap::new();
    };

    if let Some(rest) = raw.strip_prefix(VAULT_V3_MAGIC) {
        let Some(mut master_key) = try_load_existing_master_key_v3(storage_dir, is_portable) else {
            return HashMap::new();
        };
        let out = decrypt_vault_payload_v3(&master_key, rest).unwrap_or_default();
        master_key.zeroize();
        return out;
    }

    if let Some(rest) = raw.strip_prefix(VAULT_V2_MAGIC) {
        let Some(mut v2_key) = try_load_existing_master_key_v2(storage_dir, is_portable) else {
            return HashMap::new();
        };
        let migrated = decrypt_vault_payload_v1_or_v2(&v2_key, rest).unwrap_or_default();
        v2_key.zeroize();
        if !migrated.is_empty() {
            let _ = save_vault_passwords(storage_dir, is_portable, &migrated);
        }
        return migrated;
    }

    // Legacy v1 compatibility path -> upgrade automatically to PCRV3 on success
    let Some(legacy_map) = decrypt_vault_payload_v1_or_v2(&LEGACY_VAULT_KEY, &raw) else {
        return HashMap::new();
    };
    if !legacy_map.is_empty() {
        let _ = save_vault_passwords(storage_dir, is_portable, &legacy_map);
    }
    legacy_map
}

/// Encrypts and authenticates persisted connection passwords to `storage_dir/credentials.dat`
/// (`PCRV3`: AES-256-CBC + HMAC-SHA256) using a 256-bit per-installation master key protected by
/// Windows DPAPI / macOS Keychain (via stdin) or `0600` `vault_v3.key` in portable mode.
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

    let mut master_key = load_or_create_master_key_v3(storage_dir, is_portable)?;
    let (mut enc_key, mut mac_key) = derive_v3_subkeys(&master_key);
    master_key.zeroize();

    let mut pt = serde_json::to_vec(map).map_err(|e| {
        enc_key.zeroize();
        mac_key.zeroize();
        e.to_string()
    })?;
    let iv = *Uuid::new_v4().as_bytes();
    let mut buf = vec![0u8; pt.len() + 16];
    buf[..pt.len()].copy_from_slice(&pt);
    let pt_len = pt.len();
    pt.zeroize();

    let enc = Aes256CbcEnc::new_from_slices(&enc_key, &iv).map_err(|e| {
        enc_key.zeroize();
        mac_key.zeroize();
        buf.zeroize();
        e.to_string()
    })?;
    enc_key.zeroize();

    let ct_len = enc
        .encrypt_padded_mut::<Pkcs7>(&mut buf, pt_len)
        .map_err(|_| {
            mac_key.zeroize();
            "vault encrypt error".to_string()
        })?
        .len();

    let mut mac = <HmacSha256 as Mac>::new_from_slice(&mac_key).map_err(|e| {
        mac_key.zeroize();
        buf.zeroize();
        e.to_string()
    })?;
    mac_key.zeroize();
    let ct = &buf[..ct_len];
    mac.update(VAULT_V3_MAGIC);
    mac.update(&iv);
    mac.update(ct);
    let tag = mac.finalize().into_bytes();

    let mut out = Vec::with_capacity(VAULT_V3_MAGIC.len() + 16 + ct.len() + 32);
    out.extend_from_slice(VAULT_V3_MAGIC);
    out.extend_from_slice(&iv);
    out.extend_from_slice(ct);
    out.extend_from_slice(&tag);
    buf.zeroize();

    write_atomic_restricted_file(&path, &out)
}

#[cfg(test)]
mod tests {
    use super::*;

    type Aes128CbcEnc = cbc::Encryptor<Aes128>;

    #[test]
    fn test_pcrv3_vault_roundtrip_and_tamper_detection() {
        for portable in [false, true] {
            let dir = std::env::temp_dir().join(format!(
                "pacer_vault_v3_rt_{}_{}",
                portable,
                Uuid::new_v4()
            ));
            let _ = std::fs::remove_dir_all(&dir);

            let mut secrets = HashMap::new();
            secrets.insert("prof-1".to_string(), "P@ssw0rd!#1".to_string());
            secrets.insert("ssh_pwd:prof-1".to_string(), "SshSecret".to_string());

            save_vault_passwords(&dir, portable, &secrets).unwrap();
            let raw = std::fs::read(dir.join(VAULT_DATA_FILE)).unwrap();
            assert!(raw.starts_with(VAULT_V3_MAGIC));
            assert_eq!(load_vault_passwords(&dir, portable), secrets);

            // Flipping 1 bit in ciphertext must fail HMAC verification cleanly.
            let mut tampered = raw.clone();
            tampered[VAULT_V3_MAGIC.len() + 18] ^= 0x01;
            std::fs::write(dir.join(VAULT_DATA_FILE), tampered).unwrap();
            assert!(load_vault_passwords(&dir, portable).is_empty());

            // Clearing secrets removes credentials.dat cleanly
            save_vault_passwords(&dir, portable, &HashMap::new()).unwrap();
            assert!(!dir.join(VAULT_DATA_FILE).exists());
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn test_legacy_v1_and_v2_vaults_auto_migrate_to_pcrv3() {
        // 1. Legacy v1 -> PCRV3
        let dir = std::env::temp_dir().join(format!("pacer_vault_mig1_{}", Uuid::new_v4()));
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
        assert!(migrated_raw.starts_with(VAULT_V3_MAGIC));
        assert_eq!(load_vault_passwords(&dir, true), secrets);
        let _ = std::fs::remove_dir_all(&dir);

        // 2. PCRV2 (portable key file) -> PCRV3
        let dir2 = std::env::temp_dir().join(format!("pacer_vault_mig2_{}", Uuid::new_v4()));
        let _ = std::fs::remove_dir_all(&dir2);
        std::fs::create_dir_all(&dir2).unwrap();
        let v2_key = *Uuid::new_v4().as_bytes();
        std::fs::write(dir2.join(VAULT_PLAIN_KEY_FILE), v2_key).unwrap();
        let mut buf2 = vec![0u8; pt.len() + 16];
        buf2[..pt.len()].copy_from_slice(&pt);
        let enc2 = Aes128CbcEnc::new_from_slices(&v2_key, &iv).unwrap();
        let ct2 = enc2.encrypt_padded_mut::<Pkcs7>(&mut buf2, pt.len()).unwrap();
        let mut v2_bytes = Vec::with_capacity(VAULT_V2_MAGIC.len() + 16 + ct2.len());
        v2_bytes.extend_from_slice(VAULT_V2_MAGIC);
        v2_bytes.extend_from_slice(&iv);
        v2_bytes.extend_from_slice(ct2);
        std::fs::write(dir2.join(VAULT_DATA_FILE), v2_bytes).unwrap();

        assert_eq!(load_vault_passwords(&dir2, true), secrets);
        assert!(std::fs::read(dir2.join(VAULT_DATA_FILE))
            .unwrap()
            .starts_with(VAULT_V3_MAGIC));
        assert_eq!(load_vault_passwords(&dir2, true), secrets);
        let _ = std::fs::remove_dir_all(&dir2);
    }
}
