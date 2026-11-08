pub const CLSID_WRISTKEY_CREDPROV_STR: &str = "{B62F0510-9E14-4E38-96FA-8656641A5C41}";

#[cfg(windows)]
mod provider {
    use super::CLSID_WRISTKEY_CREDPROV_STR;
    use std::sync::atomic::{AtomicU32, Ordering};
    use windows::core::{GUID, HRESULT, PCWSTR};
    use windows::Win32::Foundation::*;
    use windows::Win32::System::Registry::*;
    use wristkey_core::ipc::{send_ipc_request, IpcRequest, IpcResponse};

    pub static DLL_REF_COUNT: AtomicU32 = AtomicU32::new(0);

    // {B62F0510-9E14-4E38-96FA-8656641A5C41}
    pub const CLSID_WRISTKEY_CREDPROV: GUID = GUID::from_u128(0xB62F0510_9E14_4E38_96FA_8656641A5C41);

    #[no_mangle]
    pub extern "system" fn DllCanUnloadNow() -> HRESULT {
        if DLL_REF_COUNT.load(Ordering::SeqCst) == 0 {
            S_OK
        } else {
            S_FALSE
        }
    }

    #[no_mangle]
    pub unsafe extern "system" fn DllRegisterServer() -> HRESULT {
        // Register in HKCR\CLSID\{CLSID}
        let clsid_path = format!("CLSID\\{}", CLSID_WRISTKEY_CREDPROV_STR);
        let clsid_w: Vec<u16> = clsid_path.encode_utf16().chain(std::iter::once(0)).collect();

        let mut h_clsid = HKEY::default();
        if RegCreateKeyExW(
            HKEY_CLASSES_ROOT,
            PCWSTR(clsid_w.as_ptr()),
            None,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut h_clsid,
            None,
        ).is_err() {
            return E_FAIL;
        }

        let desc_w: Vec<u16> = "WristKey Proximity Credential Provider"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let _ = RegSetValueExW(
            h_clsid,
            None,
            None,
            REG_SZ,
            Some(std::slice::from_raw_parts(desc_w.as_ptr() as *const u8, desc_w.len() * 2)),
        );

        // Subkey InprocServer32
        let inproc_path_w: Vec<u16> = "InprocServer32".encode_utf16().chain(std::iter::once(0)).collect();
        let mut h_inproc = HKEY::default();
        if RegCreateKeyExW(
            h_clsid,
            PCWSTR(inproc_path_w.as_ptr()),
            None,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut h_inproc,
            None,
        ).is_ok() {
            let mut dll_path_buf = [0u16; 512];
            let len = windows::Win32::System::LibraryLoader::GetModuleFileNameW(
                None,
                &mut dll_path_buf,
            );
            if len > 0 {
                let _ = RegSetValueExW(
                    h_inproc,
                    None,
                    None,
                    REG_SZ,
                    Some(std::slice::from_raw_parts(dll_path_buf.as_ptr() as *const u8, len as usize * 2)),
                );
            }

            let thread_model_w: Vec<u16> = "Apartment".encode_utf16().chain(std::iter::once(0)).collect();
            let thread_val_w: Vec<u16> = "ThreadingModel".encode_utf16().chain(std::iter::once(0)).collect();
            let _ = RegSetValueExW(
                h_inproc,
                PCWSTR(thread_val_w.as_ptr()),
                None,
                REG_SZ,
                Some(std::slice::from_raw_parts(thread_model_w.as_ptr() as *const u8, thread_model_w.len() * 2)),
            );
            let _ = RegCloseKey(h_inproc);
        }
        let _ = RegCloseKey(h_clsid);

        // Register in HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\{CLSID}
        let cp_path = format!(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Authentication\\Credential Providers\\{}",
            CLSID_WRISTKEY_CREDPROV_STR
        );
        let cp_w: Vec<u16> = cp_path.encode_utf16().chain(std::iter::once(0)).collect();

        let mut h_cp = HKEY::default();
        if RegCreateKeyExW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(cp_w.as_ptr()),
            None,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut h_cp,
            None,
        ).is_ok() {
            let name_w: Vec<u16> = "WristKeyCredentialProvider".encode_utf16().chain(std::iter::once(0)).collect();
            let _ = RegSetValueExW(
                h_cp,
                None,
                None,
                REG_SZ,
                Some(std::slice::from_raw_parts(name_w.as_ptr() as *const u8, name_w.len() * 2)),
            );
            let _ = RegCloseKey(h_cp);
        }

        S_OK
    }

    #[no_mangle]
    pub unsafe extern "system" fn DllUnregisterServer() -> HRESULT {
        let clsid_path = format!("CLSID\\{}", CLSID_WRISTKEY_CREDPROV_STR);
        let clsid_w: Vec<u16> = clsid_path.encode_utf16().chain(std::iter::once(0)).collect();
        let _ = RegDeleteTreeW(HKEY_CLASSES_ROOT, PCWSTR(clsid_w.as_ptr()));

        let cp_path = format!(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Authentication\\Credential Providers\\{}",
            CLSID_WRISTKEY_CREDPROV_STR
        );
        let cp_w: Vec<u16> = cp_path.encode_utf16().chain(std::iter::once(0)).collect();
        let _ = RegDeleteTreeW(HKEY_LOCAL_MACHINE, PCWSTR(cp_w.as_ptr()));

        S_OK
    }

    /// Queries the WristKey background service via Named Pipe to check proximity unlock readiness.
    pub fn check_wristkey_proximity() -> Option<wristkey_core::dpapi::UserCredentials> {
        let req = IpcRequest::RequestUnlock { nonce: 0x57524953544B4559 }; // "WRISTKEY"
        match send_ipc_request(&req, std::time::Duration::from_millis(600)) {
            Ok(IpcResponse::UnlockAuthorized { credentials, .. }) => Some(credentials),
            _ => None,
        }
    }
}

#[cfg(windows)]
pub use provider::*;
