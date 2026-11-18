use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DpapiError {
    #[error("Windows CryptProtectData failed: {0}")]
    ProtectFailed(String),
    #[error("Windows CryptUnprotectData failed: {0}")]
    UnprotectFailed(String),
    #[error("Failed to serialize or deserialize credentials: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("I/O error accessing credential vault: {0}")]
    Io(#[from] std::io::Error),
    #[error("No credentials found in vault")]
    NotFound,
    #[error("DPAPI is unsupported on non-Windows platforms")]
    UnsupportedPlatform,
}

/// User credentials required by Windows Winlogon/LogonUI to unlock the session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserCredentials {
    /// Windows domain or computer name (use "." for local accounts).
    pub domain: String,
    /// Windows user account name.
    pub username: String,
    /// Account password or PIN.
    pub password: String,
}

impl UserCredentials {
    pub fn new(domain: impl Into<String>, username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
            username: username.into(),
            password: password.into(),
        }
    }
}

/// Encrypts raw bytes using Windows Data Protection API (DPAPI).
///
/// Encryption is bound to the current Windows user and machine.
pub fn protect_data(plaintext: &[u8], entropy: Option<&[u8]>) -> Result<Vec<u8>, DpapiError> {
    #[cfg(windows)]
    {
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::LocalFree;
        use windows::Win32::Security::Cryptography::*;

        let in_blob = CRYPT_INTEGER_BLOB {
            cbData: plaintext.len() as u32,
            pbData: plaintext.as_ptr() as *mut u8,
        };

        let entropy_blob = entropy.map(|e| CRYPT_INTEGER_BLOB {
            cbData: e.len() as u32,
            pbData: e.as_ptr() as *mut u8,
        });

        let p_entropy = entropy_blob.as_ref().map(|b| b as *const _);
        let mut out_blob = CRYPT_INTEGER_BLOB::default();

        let ok = unsafe {
            CryptProtectData(
                &in_blob,
                PCWSTR::null(),
                p_entropy,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out_blob,
            )
        };

        if ok.is_ok() {
            let slice = unsafe {
                std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize)
            };
            let result = slice.to_vec();
            unsafe {
                let _ = LocalFree(Some(windows::Win32::Foundation::HLOCAL(out_blob.pbData as _)));
            }
            Ok(result)
        } else {
            let err = std::io::Error::last_os_error();
            error!("CryptProtectData failed: {}", err);
            Err(DpapiError::ProtectFailed(err.to_string()))
        }
    }

    #[cfg(not(windows))]
    {
        Err(DpapiError::UnsupportedPlatform)
    }
}

/// Decrypts DPAPI ciphertext using Windows Data Protection API.
pub fn unprotect_data(ciphertext: &[u8], entropy: Option<&[u8]>) -> Result<Vec<u8>, DpapiError> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::LocalFree;
        use windows::Win32::Security::Cryptography::*;

        let in_blob = CRYPT_INTEGER_BLOB {
            cbData: ciphertext.len() as u32,
            pbData: ciphertext.as_ptr() as *mut u8,
        };

        let entropy_blob = entropy.map(|e| CRYPT_INTEGER_BLOB {
            cbData: e.len() as u32,
            pbData: e.as_ptr() as *mut u8,
        });

        let p_entropy = entropy_blob.as_ref().map(|b| b as *const _);
        let mut out_blob = CRYPT_INTEGER_BLOB::default();

        let ok = unsafe {
            CryptUnprotectData(
                &in_blob,
                None,
                p_entropy,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out_blob,
            )
        };

        if ok.is_ok() {
            let slice = unsafe {
                std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize)
            };
            let result = slice.to_vec();
            unsafe {
                let _ = LocalFree(Some(windows::Win32::Foundation::HLOCAL(out_blob.pbData as _)));
            }
            Ok(result)
        } else {
            let err = std::io::Error::last_os_error();
            error!("CryptUnprotectData failed: {}", err);
            Err(DpapiError::UnprotectFailed(err.to_string()))
        }
    }

    #[cfg(not(windows))]
    {
        Err(DpapiError::UnsupportedPlatform)
    }
}

/// Secure storage manager for encrypted user credentials on disk.
pub struct CredentialVault {
    vault_path: PathBuf,
}

impl Default for CredentialVault {
    fn default() -> Self {
        Self::new_default()
    }
}

impl CredentialVault {
    /// Uses standard `%LOCALAPPDATA%\WristKey\credentials.vault` path.
    pub fn new_default() -> Self {
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".to_string());
        let path = PathBuf::from(local_app_data)
            .join("WristKey")
            .join("credentials.vault");
        Self { vault_path: path }
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self { vault_path: path }
    }

    pub fn vault_path(&self) -> &PathBuf {
        &self.vault_path
    }

    pub fn exists(&self) -> bool {
        self.vault_path.exists()
    }

    /// Saves user credentials securely using DPAPI.
    pub fn save(&self, creds: &UserCredentials) -> Result<(), DpapiError> {
        let json_bytes = serde_json::to_vec(creds)?;
        let entropy = b"WristKey-Session-Entropy-v1";
        let encrypted = protect_data(&json_bytes, Some(entropy))?;

        if let Some(parent) = self.vault_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&self.vault_path, encrypted)?;
        info!("Saved encrypted credentials to vault: {:?}", self.vault_path);
        Ok(())
    }

    /// Loads and decrypts user credentials from vault.
    pub fn load(&self) -> Result<UserCredentials, DpapiError> {
        if !self.exists() {
            return Err(DpapiError::NotFound);
        }

        let encrypted = fs::read(&self.vault_path)?;
        let entropy = b"WristKey-Session-Entropy-v1";
        let decrypted = unprotect_data(&encrypted, Some(entropy))?;
        let creds: UserCredentials = serde_json::from_slice(&decrypted)?;

        debug!("Successfully decrypted credentials for user: {}\\{}", creds.domain, creds.username);
        Ok(creds)
    }

    /// Deletes stored credentials from disk.
    pub fn clear(&self) -> Result<(), DpapiError> {
        if self.exists() {
            fs::remove_file(&self.vault_path)?;
            info!("Cleared credential vault: {:?}", self.vault_path);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SystemUserInfo {
    pub domain: String,
    pub username: String,
    pub msa_email: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WindowsLogonResult {
    pub success: bool,
    pub error_code: u32,
    pub message: String,
    pub detected_user: String,
    pub suggested_domain: Option<String>,
    pub suggested_username: Option<String>,
}

/// Reads the primary linked Microsoft Account email from HKCU if present.
pub fn get_msa_account_email() -> Option<String> {
    #[cfg(windows)]
    {
        use windows::core::{PCWSTR, PWSTR};
        use windows::Win32::Foundation::WIN32_ERROR;
        use windows::Win32::System::Registry::*;

        const MSA_KEY_PATH: &str = "Software\\Microsoft\\IdentityCRL\\UserExtendedProperties";
        let subkey_w: Vec<u16> = MSA_KEY_PATH.encode_utf16().chain(std::iter::once(0)).collect();

        let mut h_key = HKEY::default();
        let open_res = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey_w.as_ptr()),
                None,
                KEY_READ,
                &mut h_key,
            )
        };

        if open_res == WIN32_ERROR(0) {
            let mut name_buf = [0u16; 256];
            let mut name_len = name_buf.len() as u32;
            let enum_res = unsafe {
                RegEnumKeyExW(
                    h_key,
                    0,
                    Some(PWSTR(name_buf.as_mut_ptr())),
                    &mut name_len,
                    None,
                    None,
                    None,
                    None,
                )
            };
            unsafe {
                let _ = RegCloseKey(h_key);
            }

            if enum_res == WIN32_ERROR(0) && name_len > 0 {
                let email = String::from_utf16_lossy(&name_buf[..name_len as usize]);
                if email.contains('@') {
                    return Some(email);
                }
            }
        }
    }
    None
}

/// Returns the current Windows user domain, username, and any linked Microsoft Account email.
pub fn get_system_user_info() -> SystemUserInfo {
    let domain = std::env::var("USERDOMAIN").unwrap_or_else(|_| ".".to_string());
    let username = std::env::var("USERNAME").unwrap_or_default();
    let msa_email = get_msa_account_email();
    SystemUserInfo {
        domain,
        username,
        msa_email,
    }
}

/// Legacy helper for domain and username tuple.
pub fn get_system_username_info() -> (String, String) {
    let info = get_system_user_info();
    (info.domain, info.username)
}

/// Tests whether the provided credentials can log into Windows via native Win32 LogonUserW,
/// testing both local and Microsoft Account strategies as needed.
pub fn validate_windows_logon(domain: &str, user: &str, pass: &str) -> WindowsLogonResult {
    let sys_info = get_system_user_info();
    let detected_user = format!("{}\\{}", sys_info.domain, sys_info.username);

    #[cfg(windows)]
    {
        use windows::core::HSTRING;
        use windows::Win32::Foundation::{CloseHandle, HANDLE};
        use windows::Win32::Security::{
            LogonUserW, LOGON32_LOGON_INTERACTIVE, LOGON32_PROVIDER_DEFAULT,
        };

        let u_str = user.trim();
        let d_str = domain.trim();
        let pass_trimmed = pass.trim();

        let is_likely_pin = !pass_trimmed.is_empty()
            && pass_trimmed.chars().all(|c| c.is_ascii_digit())
            && pass_trimmed.len() <= 8;

        let try_logon = |u: &str, d: &str| -> windows::core::Result<()> {
            let user_h = HSTRING::from(u);
            let domain_final = if d.is_empty() { "." } else { d };
            let domain_h = HSTRING::from(domain_final);
            let pass_h = HSTRING::from(pass);

            let mut token = HANDLE::default();
            let res = unsafe {
                LogonUserW(
                    &user_h,
                    &domain_h,
                    &pass_h,
                    LOGON32_LOGON_INTERACTIVE,
                    LOGON32_PROVIDER_DEFAULT,
                    &mut token,
                )
            };
            if res.is_ok() {
                unsafe {
                    let _ = CloseHandle(token);
                }
            }
            res
        };

        // Strategy 1: User's exact entered credentials
        let mut last_res = try_logon(u_str, d_str);
        let mut working_domain = d_str.to_string();
        let mut working_user = u_str.to_string();

        // Strategy 2: If failed and user is an email, try domain "MicrosoftAccount"
        if last_res.is_err() && u_str.contains('@') && d_str != "MicrosoftAccount" {
            let res_msa = try_logon(u_str, "MicrosoftAccount");
            if res_msa.is_ok() {
                last_res = res_msa;
                working_domain = "MicrosoftAccount".to_string();
                working_user = u_str.to_string();
            }
        }

        // Strategy 3: If failed and a Microsoft Account email exists in registry, try that email
        if last_res.is_err() {
            if let Some(ref msa_email) = sys_info.msa_email {
                let res_msa = try_logon(msa_email, "MicrosoftAccount");
                if res_msa.is_ok() {
                    last_res = res_msa;
                    working_domain = "MicrosoftAccount".to_string();
                    working_user = msa_email.clone();
                }
            }
        }

        // Strategy 4: If domain is "." or empty, try with the actual machine domain
        if last_res.is_err() && (d_str.is_empty() || d_str == ".") {
            let res_domain = try_logon(u_str, &sys_info.domain);
            if res_domain.is_ok() {
                last_res = res_domain;
                working_domain = sys_info.domain.clone();
                working_user = u_str.to_string();
            }
        }

        match last_res {
            Ok(_) => {
                let success_msg = if working_domain == "MicrosoftAccount" {
                    format!("Autenticación correcta en Windows para cuenta Microsoft '{}'. Desbloqueo verificado.", working_user)
                } else {
                    format!("Autenticación correcta en Windows para '{}'. Desbloqueo verificado.", working_user)
                };

                WindowsLogonResult {
                    success: true,
                    error_code: 0,
                    message: success_msg,
                    detected_user,
                    suggested_domain: Some(working_domain),
                    suggested_username: Some(working_user),
                }
            }
            Err(e) => {
                let code = (e.code().0 as u32) & 0xFFFF;
                let explanation = match code {
                    1326 => {
                        if is_likely_pin {
                            "Error 1326: Has introducido un PIN. El PIN de Windows Hello (4-6 dígitos) es exclusivo de Windows Hello y no es una contraseña. Para el desbloqueo automático debes introducir la contraseña real de tu cuenta."
                        } else if sys_info.msa_email.is_some() {
                            "Error 1326: Usuario o contraseña incorrectos. Tu equipo tiene vinculada una cuenta Microsoft. Asegúrate de seleccionar 'Cuenta Microsoft' o introducir la contraseña real de tu correo Microsoft (no el PIN de Windows Hello)."
                        } else {
                            "Error 1326: Usuario o contraseña incorrectos. Verifica que el nombre de usuario coincide con tu cuenta de Windows y que has introducido tu contraseña real (no el PIN)."
                        }
                    }
                    1327 => "Error 1327: Restricción de cuenta (contraseña en blanco o restricción de horario).",
                    1330 => "Error 1330: La contraseña de la cuenta ha caducado.",
                    1331 => "Error 1331: La cuenta de usuario está deshabilitada.",
                    1909 => "Error 1909: La cuenta de usuario está bloqueada.",
                    _ => "Error al autenticar con Windows.",
                };
                WindowsLogonResult {
                    success: false,
                    error_code: code,
                    message: format!("{} (Código Windows {})", explanation, code),
                    detected_user,
                    suggested_domain: None,
                    suggested_username: None,
                }
            }
        }
    }

    #[cfg(not(windows))]
    {
        let _ = (domain, user, pass);
        WindowsLogonResult {
            success: true,
            error_code: 0,
            message: "Simulación no-Windows".to_string(),
            detected_user,
            suggested_domain: None,
            suggested_username: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn test_dpapi_protect_and_unprotect_roundtrip() {
        let original = b"SuperSecretPasscode123!@#";
        let entropy = b"CustomEntropySeed";

        let encrypted = protect_data(original, Some(entropy)).expect("Protect should succeed");
        assert_ne!(encrypted.as_slice(), original.as_slice());

        let decrypted = unprotect_data(&encrypted, Some(entropy)).expect("Unprotect should succeed");
        assert_eq!(decrypted.as_slice(), original.as_slice());
    }

    #[test]
    #[cfg(windows)]
    fn test_credential_vault_roundtrip() {
        let temp_dir = std::env::temp_dir().join(format!("wristkey_test_vault_{}", std::process::id()));
        let vault_file = temp_dir.join("test.vault");
        let vault = CredentialVault::with_path(vault_file.clone());

        let creds = UserCredentials::new(".", "TestUser", "SecretPassword456");
        vault.save(&creds).expect("Save credentials");
        assert!(vault.exists());

        let loaded = vault.load().expect("Load credentials");
        assert_eq!(loaded, creds);

        vault.clear().expect("Clear credentials");
        assert!(!vault.exists());

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    #[cfg(windows)]
    fn test_validate_windows_logon_with_invalid_password_returns_error_1326() {
        let (domain, user) = get_system_username_info();
        if !user.is_empty() {
            let res = validate_windows_logon(&domain, &user, "WrongPasswordDefinitelyInvalid123456!");
            assert!(!res.success);
            assert_eq!(res.error_code, 1326);
        }
    }
}
