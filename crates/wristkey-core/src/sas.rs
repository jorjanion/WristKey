use log::{debug, info};

/// Interacts with the Windows lock screen and secure desktop.
pub struct LockScreenController;

impl LockScreenController {
    /// Wakes up the display, resets the screen saver timer, and simulates a wake event
    /// to prompt Windows LogonUI to display the credential providers.
    pub fn wake_and_prompt_logon() {
        #[cfg(windows)]
        {
            use windows::Win32::System::Power::{
                SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED,
            };
            use windows::Win32::UI::Input::KeyboardAndMouse::*;

            // 1. Force screen on via Power Manager
            unsafe {
                SetThreadExecutionState(ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED | ES_CONTINUOUS);
            }
            debug!("Requested display power-on via SetThreadExecutionState.");

            // 2. Try SendSAS() via sas.dll if available
            if Self::try_send_sas() {
                info!("Triggered Secure Attention Sequence (SendSAS) successfully.");
                return;
            }

            // 3. Fallback: Simulate subtle user activity to dismiss lock screen wallpaper
            debug!("Simulating gentle wake key input to prompt LogonUI...");
            unsafe {
                // Key down VK_SHIFT
                let input_down = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0x10), // VK_SHIFT
                            wScan: 0,
                            dwFlags: KEYBD_EVENT_FLAGS(0),
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                let input_up = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0x10), // VK_SHIFT
                            wScan: 0,
                            dwFlags: KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };

                let inputs = [input_down, input_up];
                let sent = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
                if sent > 0 {
                    debug!("SendInput dispatched wake events.");
                } else {
                    let err = std::io::Error::last_os_error();
                    debug!("SendInput returned notice: {}", err);
                }
            }
        }

        #[cfg(not(windows))]
        {
            info!("[MOCK] Wake and prompt logon triggered.");
        }
    }

    /// Dynamically loads `sas.dll` and invokes `SendSAS(FALSE)` to simulate Ctrl+Alt+Del.
    #[cfg(windows)]
    fn try_send_sas() -> bool {
        use windows::core::{s, w};
        use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

        unsafe {
            let h_module = match LoadLibraryW(w!("sas.dll")) {
                Ok(h) => h,
                Err(_) => return false,
            };

            type SendSasFn = unsafe extern "system" fn(i32);
            let proc = GetProcAddress(h_module, s!("SendSAS"));

            if let Some(send_sas_ptr) = proc {
                let send_sas: SendSasFn = std::mem::transmute(send_sas_ptr);
                send_sas(0); // 0 = AsUser: FALSE (simulate hardware SAS)
                true
            } else {
                false
            }
        }
    }
}
