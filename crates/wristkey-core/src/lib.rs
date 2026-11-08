pub mod continuity;
pub mod crypto;
pub mod lock;
pub mod rssi;
pub mod scanner;

pub use continuity::{AppleContinuityFrame, APPLE_COMPANY_ID};
pub use crypto::{format_mac, CryptoError, IrkResolver};
pub use lock::{lock_workstation, LockError};
pub use rssi::{ProximityState, RssiTracker};
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
            require_on_wrist: true,
        }
    }
}

/// The core orchestration engine for WristKey.
pub struct WristKeyEngine {
    pub config: WristKeyConfig,
    pub irk_resolver: Option<IrkResolver>,
    pub rssi_tracker: RssiTracker,
    pub is_locked: bool,
}

impl WristKeyEngine {
    pub fn new(config: WristKeyConfig) -> Result<Self, CryptoError> {
        let irk_resolver = match &config.target_irk_hex {
            Some(hex) => Some(IrkResolver::from_hex(hex)?),
            None => None,
        };

        let rssi_tracker =
            RssiTracker::new(config.lock_threshold_dbm, config.unlock_threshold_dbm);

        Ok(Self {
            config,
            irk_resolver,
            rssi_tracker,
            is_locked: false,
        })
    }

    /// Evaluates an incoming BLE event.
    /// Returns true if this packet matched the target watch and was processed.
    pub fn handle_event(&mut self, event: &BleAdvertisementEvent) -> bool {
        // If an IRK is configured, check if this packet matches the RPA
        if let Some(ref resolver) = self.irk_resolver {
            if !resolver.matches_rpa(event.mac) {
                return false;
            }
        } else {
            // In scan/discovery mode, only accept packets with Apple manufacturer data
            if event.apple_frame.is_none() {
                return false;
            }
        }

        // Check on-wrist requirement if enabled
        if self.config.require_on_wrist {
            if let Some(ref frame) = event.apple_frame {
                if frame.message_type == continuity::TYPE_NEARBY_INFO && !frame.is_on_wrist_and_unlocked {
                    warn!(
                        "Packet matched watch {:012X}, but watch is NOT on wrist or locked (action {:?})",
                        event.mac, frame.action_code
                    );
                    return false;
                }
            }
        }

        // Update RSSI smoothing and hysteresis
        self.rssi_tracker.update(event.rssi);

        // Check auto-lock condition
        if self.config.auto_lock_enabled && !self.is_locked && self.rssi_tracker.should_lock() {
            info!("Proximity state is Far/Lost. Triggering auto-lock!");
            if lock_workstation().is_ok() {
                self.is_locked = true;
            }
        }

        // Check reset of lock state when back in Near range
        if self.is_locked && self.rssi_tracker.should_unlock() {
            info!("Device back in Near proximity. Resetting lock state.");
            self.is_locked = false;
        }

        true
    }

    /// Periodic tick to handle timeouts.
    pub fn tick(&mut self) {
        self.rssi_tracker.tick();

        if self.config.auto_lock_enabled && !self.is_locked && self.rssi_tracker.should_lock() {
            info!("Signal timeout expired. Triggering auto-lock!");
            if lock_workstation().is_ok() {
                self.is_locked = true;
            }
        }
    }
}
