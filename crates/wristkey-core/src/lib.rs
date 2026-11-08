pub mod continuity;
pub mod crypto;
pub mod device;
pub mod dpapi;
pub mod ipc;
pub mod lock;
pub mod registry;
pub mod rssi;
pub mod sas;
pub mod scanner;

pub use continuity::{AppleContinuityFrame, APPLE_COMPANY_ID};
pub use crypto::{format_mac, CryptoError, IrkResolver};
pub use device::{WatchPresenceManager, WatchSecurityStatus};
pub use dpapi::{protect_data, unprotect_data, CredentialVault, DpapiError, UserCredentials};
pub use ipc::{send_ipc_request, IpcError, IpcRequest, IpcResponse, IpcServer, WRISTKEY_PIPE_NAME};
pub use lock::{lock_workstation, LockError};
pub use registry::{is_elevated, read_paired_bluetooth_keys, PairedBluetoothKey, RegistryError};
pub use rssi::{ProximityState, RssiTracker};
pub use sas::LockScreenController;
pub use scanner::{BleAdvertisementEvent, BleScanner, ScannerError};

use log::{info, warn};
use serde::{Deserialize, Serialize};

/// High-level engine configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WristKeyConfig {
    /// 32-character hex IRK of the target Apple Watch (None means detect any Apple Watch).
    pub target_irk_hex: Option<String>,
    /// Threshold in dBm to trigger auto-lock (e.g. -82.0 dBm).
    pub lock_threshold_dbm: f64,
    /// Threshold in dBm to trigger auto-unlock (e.g. -62.0 dBm).
    pub unlock_threshold_dbm: f64,
    /// Automatically lock PC when out of range.
    pub auto_lock_enabled: bool,
    /// Automatically wake display and authorize unlock when in close proximity.
    pub auto_unlock_enabled: bool,
    /// Require Apple Watch to report on-wrist and unlocked state.
    pub require_on_wrist: bool,
}

impl Default for WristKeyConfig {
    fn default() -> Self {
        Self {
            target_irk_hex: None,
            lock_threshold_dbm: -82.0,
            unlock_threshold_dbm: -62.0,
            auto_lock_enabled: false,
            auto_unlock_enabled: false,
            require_on_wrist: true,
        }
    }
}

/// The core orchestration engine for WristKey.
pub struct WristKeyEngine {
    pub config: WristKeyConfig,
    pub presence_manager: Option<WatchPresenceManager>,
    pub rssi_tracker: RssiTracker,
    pub is_locked: bool,
}

impl WristKeyEngine {
    pub fn new(config: WristKeyConfig) -> Result<Self, CryptoError> {
        let presence_manager = if let Some(ref hex) = config.target_irk_hex {
            let resolver = IrkResolver::from_hex(hex)?;
            let mut mgr = WatchPresenceManager::new(
                resolver,
                config.lock_threshold_dbm,
                config.unlock_threshold_dbm,
            );
            mgr.require_on_wrist = config.require_on_wrist;
            Some(mgr)
        } else {
            None
        };

        let rssi_tracker =
            RssiTracker::new(config.lock_threshold_dbm, config.unlock_threshold_dbm);

        Ok(Self {
            config,
            presence_manager,
            rssi_tracker,
            is_locked: false,
        })
    }

    /// Evaluates an incoming BLE event.
    /// Returns true if this packet matched the target watch (or was an Apple packet in scan mode).
    pub fn handle_event(&mut self, event: &BleAdvertisementEvent) -> bool {
        if let Some(ref mut mgr) = self.presence_manager {
            let matched = mgr.feed_advertisement(event.mac, event.rssi, event.apple_frame.clone());
            if !matched {
                return false;
            }

            // Sync outer rssi_tracker with manager
            if let Some(rssi) = mgr.rssi_tracker.smoothed_rssi {
                self.rssi_tracker.smoothed_rssi = Some(rssi);
            }
            self.rssi_tracker.state = mgr.rssi_tracker.state;

            // Check auto-lock condition
            if self.config.auto_lock_enabled && !self.is_locked && mgr.should_lock() {
                info!("Security policy triggered workstation lock.");
                if lock_workstation().is_ok() {
                    self.is_locked = true;
                }
            }

            // Check auto-unlock proximity condition
            if self.is_locked && mgr.can_unlock() {
                info!("Authorized Apple Watch in close proximity. Initiating unlock sequence.");
                if self.config.auto_unlock_enabled {
                    LockScreenController::wake_and_prompt_logon();
                }
                self.is_locked = false;
            }

            true
        } else {
            // General discovery mode (no specific IRK configured yet)
            if event.apple_frame.is_none() {
                return false;
            }

            if self.config.require_on_wrist {
                if let Some(ref frame) = event.apple_frame {
                    if frame.message_type == continuity::TYPE_NEARBY_INFO
                        && !frame.is_on_wrist_and_unlocked
                    {
                        warn!(
                            "Apple packet from {:012X}, but not on wrist / locked (action {:?})",
                            event.mac, frame.action_code
                        );
                        return false;
                    }
                }
            }

            self.rssi_tracker.update(event.rssi);

            if self.config.auto_lock_enabled && !self.is_locked && self.rssi_tracker.should_lock() {
                info!("Proximity state is Far/Lost. Triggering auto-lock.");
                if lock_workstation().is_ok() {
                    self.is_locked = true;
                }
            }

            if self.is_locked && self.rssi_tracker.should_unlock() {
                if self.config.auto_unlock_enabled {
                    LockScreenController::wake_and_prompt_logon();
                }
                self.is_locked = false;
            }

            true
        }
    }

    /// Periodic tick to handle timeouts.
    pub fn tick(&mut self) {
        if let Some(ref mut mgr) = self.presence_manager {
            mgr.tick();
            if self.config.auto_lock_enabled && !self.is_locked && mgr.should_lock() {
                info!("Device timeout or lock state detected. Locking workstation.");
                if lock_workstation().is_ok() {
                    self.is_locked = true;
                }
            }
        } else {
            self.rssi_tracker.tick();
            if self.config.auto_lock_enabled && !self.is_locked && self.rssi_tracker.should_lock() {
                info!("Signal timeout expired. Triggering auto-lock.");
                if lock_workstation().is_ok() {
                    self.is_locked = true;
                }
            }
        }
    }
}
