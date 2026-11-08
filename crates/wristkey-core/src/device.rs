use crate::continuity::AppleContinuityFrame;
use crate::crypto::IrkResolver;
use crate::rssi::{ProximityState, RssiTracker};
use log::{debug, info, warn};
use std::time::{Duration, Instant};

/// Security status of an Apple Watch presence session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchSecurityStatus {
    /// Watch not yet discovered or not matching IRK.
    NotDetected,
    /// Watch detected nearby, but it is locked or NOT on wrist.
    DetectedUnauthorized { reason: String },
    /// Watch detected, on wrist, unlocked, but outside the close unlock distance.
    AuthorizedFar,
    /// Watch detected, on wrist, unlocked, and inside close proximity (ready to unlock).
    AuthorizedInProximity,
}

/// Comprehensive Apple Watch presence and state tracker.
pub struct WatchPresenceManager {
    /// Cryptographic IRK resolver for the target Apple Watch.
    pub irk_resolver: IrkResolver,
    /// Smoothed RSSI signal and distance tracker.
    pub rssi_tracker: RssiTracker,
    /// Most recent rotating private MAC address (changes every 15m).
    pub current_rpa_mac: Option<u64>,
    /// Last decoded Apple Continuity telemetry frame.
    pub last_frame: Option<AppleContinuityFrame>,
    /// Timestamp when the watch was last confirmed on wrist and unlocked.
    pub last_authorized_at: Option<Instant>,
    /// Require Apple Continuity Nearby Info on-wrist confirmation.
    pub require_on_wrist: bool,
    /// Maximum duration allowed since last valid on-wrist packet before revoking authorization.
    pub on_wrist_timeout: Duration,
}

impl WatchPresenceManager {
    pub fn new(
        irk_resolver: IrkResolver,
        lock_threshold_dbm: f64,
        unlock_threshold_dbm: f64,
    ) -> Self {
        Self {
            irk_resolver,
            rssi_tracker: RssiTracker::new(lock_threshold_dbm, unlock_threshold_dbm),
            current_rpa_mac: None,
            last_frame: None,
            last_authorized_at: None,
            require_on_wrist: true,
            on_wrist_timeout: Duration::from_secs(6),
        }
    }

    /// Feeds an incoming BLE advertisement.
    /// Returns true if this advertisement belongs to the tracked Apple Watch.
    pub fn feed_advertisement(
        &mut self,
        mac: u64,
        rssi: i16,
        apple_frame: Option<AppleContinuityFrame>,
    ) -> bool {
        // Step 1: Cryptographic IRK check
        if !self.irk_resolver.matches_rpa(mac) {
            return false;
        }

        self.current_rpa_mac = Some(mac);
        self.rssi_tracker.update(rssi);

        let now = Instant::now();

        // Step 2: Apple Continuity Telemetry evaluation
        if let Some(frame) = apple_frame {
            debug!(
                "Matched Apple Watch {:012X} [RSSI: {} dBm] -> Action: {:?}, OnWrist: {}, Locked: {}",
                mac, rssi, frame.action_name, frame.is_on_wrist, frame.is_locked
            );

            if frame.is_on_wrist_and_unlocked {
                self.last_authorized_at = Some(now);
            } else if frame.is_locked {
                warn!("Apple Watch {:012X} reported LOCKED state!", mac);
                self.last_authorized_at = None;
            }

            self.last_frame = Some(frame);
        } else {
            // Received a packet from the watch (matching IRK), but not a Nearby Info frame
            // If recently authorized within grace window, retain state
            if let Some(last_auth) = self.last_authorized_at {
                if now.duration_since(last_auth) > self.on_wrist_timeout {
                    self.last_authorized_at = None;
                }
            }
        }

        true
    }

    /// Evaluates current security status.
    pub fn security_status(&self) -> WatchSecurityStatus {
        let now = Instant::now();

        // Check if device is seen recently
        if self.rssi_tracker.last_seen.is_none() || self.rssi_tracker.state == ProximityState::Lost {
            return WatchSecurityStatus::NotDetected;
        }

        // Check on-wrist requirement
        if self.require_on_wrist {
            if let Some(ref frame) = self.last_frame {
                if frame.is_locked {
                    return WatchSecurityStatus::DetectedUnauthorized {
                        reason: "Watch is locked with passcode".to_string(),
                    };
                }
            }

            if let Some(auth_time) = self.last_authorized_at {
                if now.duration_since(auth_time) > self.on_wrist_timeout {
                    return WatchSecurityStatus::DetectedUnauthorized {
                        reason: "Watch on-wrist telemetry timed out (removed from wrist?)".to_string(),
                    };
                }
            } else {
                return WatchSecurityStatus::DetectedUnauthorized {
                    reason: "Watch has not confirmed on-wrist and unlocked state".to_string(),
                };
            }
        }

        // Evaluate distance
        match self.rssi_tracker.state {
            ProximityState::Near => WatchSecurityStatus::AuthorizedInProximity,
            ProximityState::Far => WatchSecurityStatus::AuthorizedFar,
            _ => WatchSecurityStatus::AuthorizedFar,
        }
    }

    /// Checks if all security criteria are satisfied to authorize an automatic session unlock.
    pub fn can_unlock(&self) -> bool {
        self.security_status() == WatchSecurityStatus::AuthorizedInProximity
    }

    /// Checks if workstation should be locked.
    ///
    /// Triggers if:
    /// - Device moved to Far state for longer than the grace period, OR
    /// - Signal was completely lost, OR
    /// - The watch was removed from the wrist / locked while in range!
    pub fn should_lock(&self) -> bool {
        // Case 1: Standard distance / timeout lock
        if self.rssi_tracker.should_lock() {
            return true;
        }

        // Case 2: Watch explicitly removed or locked while still transmitting nearby
        if self.require_on_wrist {
            if let Some(ref frame) = self.last_frame {
                if frame.is_locked {
                    info!("Lock condition met: Apple Watch was locked by user or removed from wrist.");
                    return true;
                }
            }
        }

        false
    }

    /// Periodic background tick (handles timeouts).
    pub fn tick(&mut self) {
        self.rssi_tracker.tick();

        if let Some(last_auth) = self.last_authorized_at {
            if Instant::now().duration_since(last_auth) > self.on_wrist_timeout {
                self.last_authorized_at = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::IrkResolver;

    #[test]
    fn test_presence_manager_flow() {
        // Bluetooth SIG test IRK
        let irk: [u8; 16] = [
            0xec, 0x02, 0x34, 0xa3, 0x57, 0xc8, 0xad, 0x05,
            0x34, 0x10, 0x10, 0xa6, 0x0a, 0x39, 0x7d, 0x9b,
        ];
        let resolver = IrkResolver::new(&irk);
        let prand = [0x70, 0x81, 0x94];
        let hash = [0x0d, 0xfb, 0xaa];
        let valid_mac: u64 = ((prand[0] as u64) << 40)
            | ((prand[1] as u64) << 32)
            | ((prand[2] as u64) << 24)
            | ((hash[0] as u64) << 16)
            | ((hash[1] as u64) << 8)
            | (hash[2] as u64);

        let mut mgr = WatchPresenceManager::new(resolver, -80.0, -60.0);

        // Foreign MAC is ignored
        assert!(!mgr.feed_advertisement(valid_mac ^ 0x01, -55, None));
        assert_eq!(mgr.security_status(), WatchSecurityStatus::NotDetected);

        // Valid MAC with On-Wrist Nearby Info at close distance (-55 dBm)
        let frame = AppleContinuityFrame::parse(&[0x10, 0x05, 0x1A, 0x44, 0x11, 0x22, 0x33]);
        assert!(mgr.feed_advertisement(valid_mac, -55, frame));
        assert!(mgr.can_unlock());
        assert_eq!(mgr.security_status(), WatchSecurityStatus::AuthorizedInProximity);

        // If watch locks (e.g. taken off wrist), cannot unlock and triggers lock!
        let locked_frame = AppleContinuityFrame::parse(&[0x10, 0x05, 0x03, 0x64, 0x11, 0x22, 0x33]);
        assert!(mgr.feed_advertisement(valid_mac, -55, locked_frame));
        assert!(!mgr.can_unlock());
        assert!(mgr.should_lock());
    }
}
