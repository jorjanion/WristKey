use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("Access denied. Reading Bluetooth pairing keys requires elevated (Administrator/SYSTEM) privileges.")]
    AccessDenied,
    #[error("BTHPORT Keys registry path not found: {0}")]
    PathNotFound(String),
    #[error("Windows Registry error (code {0}): {1}")]
    Win32(u32, String),
    #[error("Registry operation unsupported on non-Windows platforms")]
    UnsupportedPlatform,
}

/// Information about a paired Bluetooth device stored in Windows Registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairedBluetoothKey {
    /// Host Bluetooth controller MAC address (hex string).
    pub host_adapter_mac: String,
    /// Paired peripheral MAC address (hex string).
    pub device_mac: String,
    /// Identity Resolving Key (IRK) if available (16 bytes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub irk: Option<[u8; 16]>,
    /// IRK formatted as a 32-character hexadecimal string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub irk_hex: Option<String>,
    /// Long Term Key (LTK) for BLE encryption if available (16 bytes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ltk: Option<[u8; 16]>,
    /// Legacy BR/EDR Link Key if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_key: Option<[u8; 16]>,
    /// Friendly name or device class if resolvable.
    pub name: Option<String>,
}

impl fmt::Display for PairedBluetoothKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Device: {} (Host: {})",
            self.device_mac, self.host_adapter_mac
        )?;
        if let Some(ref irk_hex) = self.irk_hex {
            write!(f, " | IRK: {}", irk_hex)?;
        }
        if self.ltk.is_some() {
            write!(f, " | [LTK present]")?;
        }
        if self.link_key.is_some() {
            write!(f, " | [LinkKey present]")?;
        }
        Ok(())
    }
}

/// Checks whether the current process is running with elevated (Administrator) privileges.
pub fn is_elevated() -> bool {
    #[cfg(windows)]
    {
        use windows::Win32::Security::*;
        use windows::Win32::System::Threading::*;

        unsafe {
            let mut token = windows::Win32::Foundation::HANDLE::default();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
                return false;
            }

            let mut elevation = TOKEN_ELEVATION::default();
            let mut returned_len = 0u32;
            let res = GetTokenInformation(
                token,
                TokenElevation,
                Some(&mut elevation as *mut _ as *mut _),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut returned_len,
            );

            let _ = windows::Win32::Foundation::CloseHandle(token);

            res.is_ok() && elevation.TokenIsElevated != 0
        }
    }

    #[cfg(not(windows))]
    {
        false
    }
}

/// Reads paired Bluetooth keys from `HKLM\SYSTEM\CurrentControlSet\Services\BTHPORT\Parameters\Keys`.
///
/// NOTE: This registry location is protected by Windows ACLs and requires Administrator or SYSTEM privileges.
pub fn read_paired_bluetooth_keys() -> Result<Vec<PairedBluetoothKey>, RegistryError> {
    #[cfg(windows)]
    {
        use windows::core::{PCWSTR, PWSTR};
        use windows::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, WIN32_ERROR};
        use windows::Win32::System::Registry::*;

        const BTHPORT_KEYS_PATH: &str = "SYSTEM\\CurrentControlSet\\Services\\BTHPORT\\Parameters\\Keys";
        let subkey_w: Vec<u16> = BTHPORT_KEYS_PATH.encode_utf16().chain(std::iter::once(0)).collect();

        let mut h_keys = HKEY::default();
        let open_res = unsafe {
            RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                PCWSTR(subkey_w.as_ptr()),
                None,
                KEY_READ,
                &mut h_keys,
            )
        };

        if open_res == ERROR_ACCESS_DENIED {
            return Err(RegistryError::AccessDenied);
        } else if open_res == ERROR_FILE_NOT_FOUND {
            return Err(RegistryError::PathNotFound(BTHPORT_KEYS_PATH.to_string()));
        } else if open_res != WIN32_ERROR(0) {
            return Err(RegistryError::Win32(open_res.0, format!("RegOpenKeyExW failed: {}", open_res.0)));
        }

        let mut results = Vec::new();

        // Enumerate host adapter subkeys (e.g. "001a7dda710a")
        let mut adapter_index = 0u32;
        loop {
            let mut adapter_name_buf = [0u16; 256];
            let mut adapter_name_len = adapter_name_buf.len() as u32;

            let enum_res = unsafe {
                RegEnumKeyExW(
                    h_keys,
                    adapter_index,
                    Some(PWSTR(adapter_name_buf.as_mut_ptr())),
                    &mut adapter_name_len,
                    None,
                    None,
                    None,
                    None,
                )
            };

            if enum_res != WIN32_ERROR(0) {
                break; // No more adapter keys
            }

            let adapter_mac = String::from_utf16_lossy(&adapter_name_buf[..adapter_name_len as usize]);
            debug!("Found Bluetooth host adapter in registry: {}", adapter_mac);

            // Open adapter key
            let adapter_w: Vec<u16> = adapter_mac.encode_utf16().chain(std::iter::once(0)).collect();
            let mut h_adapter = HKEY::default();
            let open_adapter = unsafe {
                RegOpenKeyExW(
                    h_keys,
                    PCWSTR(adapter_w.as_ptr()),
                    None,
                    KEY_READ,
                    &mut h_adapter,
                )
            };

            if open_adapter == WIN32_ERROR(0) {
                // 1. Check for BLE device subkeys under adapter key (e.g. "d8b377c8e9f2")
                let mut dev_index = 0u32;
                loop {
                    let mut dev_name_buf = [0u16; 256];
                    let mut dev_name_len = dev_name_buf.len() as u32;

                    let enum_dev = unsafe {
                        RegEnumKeyExW(
                            h_adapter,
                            dev_index,
                            Some(PWSTR(dev_name_buf.as_mut_ptr())),
                            &mut dev_name_len,
                            None,
                            None,
                            None,
                            None,
                        )
                    };

                    if enum_dev != WIN32_ERROR(0) {
                        break;
                    }

                    let dev_mac = String::from_utf16_lossy(&dev_name_buf[..dev_name_len as usize]);
                    debug!("Found BLE peripheral key subkey: {}", dev_mac);

                    // Open device subkey to read IRK / LTK
                    let dev_w: Vec<u16> = dev_mac.encode_utf16().chain(std::iter::once(0)).collect();
                    let mut h_dev = HKEY::default();
                    if unsafe { RegOpenKeyExW(h_adapter, PCWSTR(dev_w.as_ptr()), None, KEY_READ, &mut h_dev) } == WIN32_ERROR(0) {
                        let irk = read_binary_value(h_dev, "IRK");
                        let ltk = read_binary_value(h_dev, "LTK");

                        let irk_hex = irk.as_ref().map(|bytes| hex_encode(bytes));

                        results.push(PairedBluetoothKey {
                            host_adapter_mac: adapter_mac.clone(),
                            device_mac: dev_mac,
                            irk,
                            irk_hex,
                            ltk,
                            link_key: None,
                            name: None,
                        });

                        unsafe { let _ = RegCloseKey(h_dev); };
                    }

                    dev_index += 1;
                }

                // 2. Also check direct values on adapter key (legacy BR/EDR link keys)
                let mut val_index = 0u32;
                loop {
                    let mut val_name_buf = [0u16; 256];
                    let mut val_name_len = val_name_buf.len() as u32;
                    let mut val_type = 0u32;
                    let mut data_buf = [0u8; 64];
                    let mut data_len = data_buf.len() as u32;

                    let enum_val = unsafe {
                        RegEnumValueW(
                            h_adapter,
                            val_index,
                            Some(PWSTR(val_name_buf.as_mut_ptr())),
                            &mut val_name_len,
                            None,
                            Some(&mut val_type),
                            Some(data_buf.as_mut_ptr()),
                            Some(&mut data_len),
                        )
                    };

                    if enum_val != WIN32_ERROR(0) {
                        break;
                    }

                    let val_name = String::from_utf16_lossy(&val_name_buf[..val_name_len as usize]);
                    if val_type == REG_BINARY.0 && data_len == 16 {
                        let mut key_bytes = [0u8; 16];
                        key_bytes.copy_from_slice(&data_buf[..16]);

                        // Only add if not already captured as BLE
                        if !results.iter().any(|r| r.device_mac.eq_ignore_ascii_case(&val_name)) {
                            results.push(PairedBluetoothKey {
                                host_adapter_mac: adapter_mac.clone(),
                                device_mac: val_name,
                                irk: None,
                                irk_hex: None,
                                ltk: None,
                                link_key: Some(key_bytes),
                                name: None,
                            });
                        }
                    }

                    val_index += 1;
                }

                unsafe { let _ = RegCloseKey(h_adapter); };
            }

            adapter_index += 1;
        }

        unsafe { let _ = RegCloseKey(h_keys); };

        info!("Extracted {} paired Bluetooth key(s) from Windows Registry.", results.len());
        Ok(results)
    }

    #[cfg(not(windows))]
    {
        Err(RegistryError::UnsupportedPlatform)
    }
}

#[cfg(windows)]
fn read_binary_value(hkey: windows::Win32::System::Registry::HKEY, val_name: &str) -> Option<[u8; 16]> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::WIN32_ERROR;
    use windows::Win32::System::Registry::*;

    let val_w: Vec<u16> = val_name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut val_type = REG_VALUE_TYPE::default();
    let mut buf = [0u8; 64];
    let mut buf_len = buf.len() as u32;

    let res = unsafe {
        RegQueryValueExW(
            hkey,
            PCWSTR(val_w.as_ptr()),
            None,
            Some(&mut val_type),
            Some(buf.as_mut_ptr()),
            Some(&mut buf_len),
        )
    };

    if res == WIN32_ERROR(0) && val_type == REG_BINARY && buf_len == 16 {
        let mut out = [0u8; 16];
        out.copy_from_slice(&buf[..16]);
        Some(out)
    } else {
        None
    }
}

pub fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{:02x}", b);
    }
    s
}
