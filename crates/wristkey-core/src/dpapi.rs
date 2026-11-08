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
}
