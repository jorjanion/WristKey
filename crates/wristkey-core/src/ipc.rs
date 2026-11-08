use crate::dpapi::{CredentialVault, UserCredentials};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

pub const WRISTKEY_PIPE_NAME: &str = r"\\.\pipe\WristKeyUnlockPipe";

#[derive(Error, Debug)]
pub enum IpcError {
    #[error("I/O error communicating with Named Pipe: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Pipe connection timed out")]
    Timeout,
    #[error("IPC unsupported on this platform")]
    UnsupportedPlatform,
}

/// Request sent from Credential Provider (in LogonUI) to the WristKey Daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcRequest {
    Ping,
    GetWatchState,
    RequestUnlock { nonce: u64 },
}

/// Response sent from WristKey Daemon to the Credential Provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcResponse {
    Pong,
    WatchState {
        is_ready: bool,
        smoothed_rssi: Option<f64>,
        is_on_wrist: bool,
        is_locked: bool,
    },
    UnlockAuthorized {
        credentials: UserCredentials,
        nonce_echo: u64,
    },
    UnlockDenied {
        reason: String,
    },
}

/// Helper function to send an IPC request from a client (e.g. Credential Provider) to the server.
pub fn send_ipc_request(request: &IpcRequest, timeout: Duration) -> Result<IpcResponse, IpcError> {
    #[cfg(windows)]
    {
        use std::fs::OpenOptions;

        let start = std::time::Instant::now();
        let mut pipe = loop {
            match OpenOptions::new()
                .read(true)
                .write(true)
                .open(WRISTKEY_PIPE_NAME)
            {
                Ok(file) => break file,
                Err(_e) => {
                    if start.elapsed() >= timeout {
                        return Err(IpcError::Timeout);
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        };

        // Write length-prefixed JSON request
        let req_bytes = serde_json::to_vec(request)?;
        let len_bytes = (req_bytes.len() as u32).to_le_bytes();
        pipe.write_all(&len_bytes)?;
        pipe.write_all(&req_bytes)?;
        pipe.flush()?;

        // Read response length
        let mut resp_len_buf = [0u8; 4];
        pipe.read_exact(&mut resp_len_buf)?;
        let resp_len = u32::from_le_bytes(resp_len_buf) as usize;

        let mut resp_bytes = vec![0u8; resp_len];
        pipe.read_exact(&mut resp_bytes)?;

        let response: IpcResponse = serde_json::from_slice(&resp_bytes)?;
        Ok(response)
    }

    #[cfg(not(windows))]
    {
        Err(IpcError::UnsupportedPlatform)
    }
}

/// Runs the background Named Pipe IPC server.
pub struct IpcServer {
    is_running: Arc<AtomicBool>,
}

impl Default for IpcServer {
    fn default() -> Self {
        Self::new()
    }
}

impl IpcServer {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Spawns the Named Pipe listener in a background thread.
    ///
    /// The `status_provider` closure returns `(is_ready_to_unlock, smoothed_rssi, is_on_wrist, is_locked)`.
    pub fn start<F>(&self, status_provider: F) -> Result<(), IpcError>
    where
        F: Fn() -> (bool, Option<f64>, bool, bool) + Send + Sync + 'static,
    {
        #[cfg(windows)]
        {
            use windows::core::PCWSTR;
            use windows::Win32::Foundation::*;
            use windows::Win32::Storage::FileSystem::*;
            use windows::Win32::System::Pipes::*;

            let is_running = self.is_running.clone();
            is_running.store(true, Ordering::SeqCst);

            let pipe_name_w: Vec<u16> = WRISTKEY_PIPE_NAME
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            let provider = Arc::new(status_provider);

            std::thread::spawn(move || {
                info!("WristKey Named Pipe IPC Server listening on {}", WRISTKEY_PIPE_NAME);
                let vault = CredentialVault::new_default();

                while is_running.load(Ordering::SeqCst) {
                    let h_pipe = unsafe {
                        CreateNamedPipeW(
                            PCWSTR(pipe_name_w.as_ptr()),
                            PIPE_ACCESS_DUPLEX,
                            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                            PIPE_UNLIMITED_INSTANCES,
                            4096,
                            4096,
                            0,
                            None,
                        )
                    };

                    if h_pipe == INVALID_HANDLE_VALUE {
                        let err = std::io::Error::last_os_error();
                        error!("CreateNamedPipeW error: {}", err);
                        std::thread::sleep(Duration::from_millis(500));
                        continue;
                    }

                    // Wait for client connection
                    let connected = unsafe { ConnectNamedPipe(h_pipe, None) };
                    let client_connected = connected.is_ok()
                        || unsafe { windows::Win32::Foundation::GetLastError() }
                            == ERROR_PIPE_CONNECTED;

                    if client_connected {
                        debug!("Client connected to WristKey Named Pipe.");
                        Self::handle_client(h_pipe, &provider, &vault);
                    }

                    unsafe {
                        let _ = DisconnectNamedPipe(h_pipe);
                        let _ = CloseHandle(h_pipe);
                    }
                }

                info!("WristKey Named Pipe IPC Server stopped.");
            });

            Ok(())
        }

        #[cfg(not(windows))]
        {
            Err(IpcError::UnsupportedPlatform)
        }
    }

    #[cfg(windows)]
    fn handle_client<F>(
        h_pipe: windows::Win32::Foundation::HANDLE,
        provider: &Arc<F>,
        vault: &CredentialVault,
    ) where
        F: Fn() -> (bool, Option<f64>, bool, bool),
    {
        use windows::Win32::Storage::FileSystem::{ReadFile, WriteFile};

        // Read 4-byte length prefix
        let mut len_buf = [0u8; 4];
        let mut bytes_read = 0u32;
        let ok = unsafe {
            ReadFile(
                h_pipe,
                Some(&mut len_buf),
                Some(&mut bytes_read),
                None,
            )
        };

        if ok.is_err() || bytes_read < 4 {
            return;
        }

        let req_len = u32::from_le_bytes(len_buf) as usize;
        let mut req_buf = vec![0u8; req_len];
        let ok = unsafe {
            ReadFile(
                h_pipe,
                Some(&mut req_buf),
                Some(&mut bytes_read),
                None,
            )
        };

        if ok.is_err() || (bytes_read as usize) < req_len {
            return;
        }

        let request: Result<IpcRequest, _> = serde_json::from_slice(&req_buf);
        let response = match request {
            Ok(IpcRequest::Ping) => IpcResponse::Pong,
            Ok(IpcRequest::GetWatchState) => {
                let (ready, rssi, on_wrist, locked) = provider();
                IpcResponse::WatchState {
                    is_ready: ready,
                    smoothed_rssi: rssi,
                    is_on_wrist: on_wrist,
                    is_locked: locked,
                }
            }
            Ok(IpcRequest::RequestUnlock { nonce }) => {
                let (ready, _rssi, on_wrist, locked) = provider();
                if !ready || !on_wrist || locked {
                    IpcResponse::UnlockDenied {
                        reason: "Apple Watch not in proximity or not authenticated on wrist.".to_string(),
                    }
                } else {
                    match vault.load() {
                        Ok(creds) => IpcResponse::UnlockAuthorized {
                            credentials: creds,
                            nonce_echo: nonce,
                        },
                        Err(e) => IpcResponse::UnlockDenied {
                            reason: format!("Failed to read credential vault: {}", e),
                        },
                    }
                }
            }
            Err(e) => IpcResponse::UnlockDenied {
                reason: format!("Invalid IPC request: {}", e),
            },
        };

        if let Ok(resp_bytes) = serde_json::to_vec(&response) {
            let resp_len = (resp_bytes.len() as u32).to_le_bytes();
            let mut bytes_written = 0u32;
            unsafe {
                let _ = WriteFile(
                    h_pipe,
                    Some(&resp_len),
                    Some(&mut bytes_written),
                    None,
                );
                let _ = WriteFile(
                    h_pipe,
                    Some(&resp_bytes),
                    Some(&mut bytes_written),
                    None,
                );
            }
        }
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }
}
