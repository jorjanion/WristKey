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
