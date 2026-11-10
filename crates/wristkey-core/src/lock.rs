use log::{error, info};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LockError {
    #[error("Windows API LockWorkStation failed")]
    LockFailed,
    #[error("LockWorkStation is not supported on this operating system")]
    UnsupportedPlatform,
}

/// Locks the current interactive Windows desktop session immediately.
///
/// Calls the native `user32!LockWorkStation()` API.
pub fn lock_workstation() -> Result<(), LockError> {
    #[cfg(windows)]
    {
        #[link(name = "user32")]
        extern "system" {
            fn LockWorkStation() -> i32;
        }

        info!("Triggering Windows workstation lock (LockWorkStation)...");
        let result = unsafe { LockWorkStation() };
        if result != 0 {
            info!("Workstation locked successfully.");
            Ok(())
        } else {
            let err = std::io::Error::last_os_error();
            error!("LockWorkStation returned error: {}", err);
            Err(LockError::LockFailed)
        }
    }

    #[cfg(not(windows))]
    {
        info!("[MOCK] Workstation lock triggered (simulated non-windows platform).");
        Ok(())
    }
}

/// Detects whether the current Windows desktop is locked (showing Lock Screen / LogonUI).
///
/// Uses `user32!OpenInputDesktop(0, FALSE, DESKTOP_SWITCHDESKTOP)`. If the workstation is
/// locked, the active input desktop is switched to Winlogon, causing `OpenInputDesktop`
/// to fail and return NULL (0) for non-SYSTEM processes.
pub fn is_workstation_locked() -> bool {
    #[cfg(windows)]
    {
        #[link(name = "user32")]
        extern "system" {
            fn OpenInputDesktop(dw_flags: u32, f_inherit: i32, dw_desired_access: u32) -> isize;
            fn CloseDesktop(h_desktop: isize) -> i32;
        }

        const DESKTOP_SWITCHDESKTOP: u32 = 0x0100;

        unsafe {
            let h_desktop = OpenInputDesktop(0, 0, DESKTOP_SWITCHDESKTOP);
            if h_desktop == 0 {
                true
            } else {
                CloseDesktop(h_desktop);
                false
            }
        }
    }

    #[cfg(not(windows))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_workstation_locked_callable() {
        // Must be callable without crashing or panicking
        let _ = is_workstation_locked();
    }
}
