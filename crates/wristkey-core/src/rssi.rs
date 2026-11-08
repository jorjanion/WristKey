use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Proximity states calculated from smoothed RSSI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProximityState {
    Unknown,
    Near,
    Far,
    Lost,
}

impl std::fmt::Display for ProximityState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "Unknown"),
            Self::Near => write!(f, "Near (In Range)"),
            Self::Far => write!(f, "Far (Out of Range)"),
            Self::Lost => write!(f, "Lost (No Signal)"),
        }
    }
}

/// Signal tracker using Exponential Moving Average (EMA) and a dual-threshold hysteresis machine.
#[derive(Debug, Clone)]
pub struct RssiTracker {
    /// Smoothed RSSI value in dBm.
    pub smoothed_rssi: Option<f64>,
    /// Exponential Moving Average smoothing factor (0.0 to 1.0, e.g. 0.25).
    pub alpha: f64,
    /// Threshold in dBm below which the device is considered "Far" (triggering lock).
    pub lock_threshold_dbm: f64,
    /// Threshold in dBm above which the device is considered "Near" (triggering unlock).
    pub unlock_threshold_dbm: f64,
    /// Maximum duration allowed without receiving packets before declaring the signal "Lost".
    pub signal_timeout: Duration,
    /// Grace period before triggering an auto-lock when in Far state.
    pub lock_grace_period: Duration,
    /// Timestamp of the last received packet.
    pub last_seen: Option<Instant>,
    /// Timestamp when the signal first fell below the lock threshold.
    pub far_since: Option<Instant>,
    /// Current hysteresis proximity state.
    pub state: ProximityState,
    /// Total packet count processed.
    pub packet_count: u64,
}

impl Default for RssiTracker {
    fn default() -> Self {
        Self::new(-82.0, -62.0)
    }
}

impl RssiTracker {
    pub fn new(lock_threshold_dbm: f64, unlock_threshold_dbm: f64) -> Self {
        Self {
            smoothed_rssi: None,
            alpha: 0.25,
            lock_threshold_dbm,
            unlock_threshold_dbm,
            signal_timeout: Duration::from_secs(5),
            lock_grace_period: Duration::from_millis(2500),
            last_seen: None,
            far_since: None,
            state: ProximityState::Unknown,
            packet_count: 0,
        }
    }

    /// Feeds a new raw RSSI measurement and updates the state.
    pub fn update(&mut self, raw_rssi: i16) -> ProximityState {
        let now = Instant::now();
        self.last_seen = Some(now);
        self.packet_count += 1;

        let raw_f = raw_rssi as f64;
        let smoothed = match self.smoothed_rssi {
            Some(prev) => self.alpha * raw_f + (1.0 - self.alpha) * prev,
            None => raw_f,
        };
        self.smoothed_rssi = Some(smoothed);

        // Hysteresis evaluation
        match self.state {
            ProximityState::Unknown | ProximityState::Lost => {
                if smoothed >= self.unlock_threshold_dbm {
                    self.state = ProximityState::Near;
                    self.far_since = None;
                } else if smoothed <= self.lock_threshold_dbm {
                    self.state = ProximityState::Far;
                    if self.far_since.is_none() {
                        self.far_since = Some(now);
                    }
                }
            }
            ProximityState::Near => {
                if smoothed <= self.lock_threshold_dbm {
                    self.state = ProximityState::Far;
                    if self.far_since.is_none() {
                        self.far_since = Some(now);
                    }
                }
            }
            ProximityState::Far => {
                if smoothed >= self.unlock_threshold_dbm {
                    self.state = ProximityState::Near;
                    self.far_since = None;
                }
            }
        }

        self.state
    }

    /// Periodic tick to check for timeouts. Call this periodically (e.g. every 500ms).
    pub fn tick(&mut self) -> ProximityState {
        let now = Instant::now();
        if let Some(last) = self.last_seen {
            if now.duration_since(last) > self.signal_timeout {
                self.state = ProximityState::Lost;
                if self.far_since.is_none() {
                    self.far_since = Some(now);
                }
            }
        } else {
            self.state = ProximityState::Unknown;
        }

        self.state
    }

    /// Checks whether an auto-lock action should be triggered right now.
    ///
    /// Requires being in Far or Lost state continuously for longer than the grace period.
    pub fn should_lock(&self) -> bool {
        if matches!(self.state, ProximityState::Far | ProximityState::Lost) {
            if let Some(since) = self.far_since {
                return Instant::now().duration_since(since) >= self.lock_grace_period;
            }
        }
        false
    }

    /// Checks whether an auto-unlock action can be triggered.
    pub fn should_unlock(&self) -> bool {
        self.state == ProximityState::Near
    }

    /// Approximate distance in meters using the standard Log-Distance Path Loss model.
    /// Assumes reference RSSI at 1m is -59 dBm and path-loss exponent is 2.5.
    pub fn estimated_distance_meters(&self) -> Option<f64> {
        self.smoothed_rssi.map(|rssi| {
            let measured_power = -59.0;
            let n = 2.5;
            let ratio = (measured_power - rssi) / (10.0 * n);
            10.0_f64.powf(ratio)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rssi_smoothing_and_hysteresis() {
        let mut tracker = RssiTracker::new(-80.0, -65.0);

        // First packet at -60 dBm (Strong) -> transitions to Near
        let state = tracker.update(-60);
        assert_eq!(state, ProximityState::Near);
        assert!((tracker.smoothed_rssi.unwrap() - (-60.0)).abs() < 0.01);

        // Small dip to -70 dBm (within hysteresis gap) -> remains Near
        let state = tracker.update(-70);
        assert_eq!(state, ProximityState::Near);

        // Multiple weak packets taking smoothed value below -80 dBm -> transitions to Far
        for _ in 0..10 {
            tracker.update(-88);
        }
        assert_eq!(tracker.state, ProximityState::Far);

        // Single strong packet at -64 dBm -> transitions back to Near
        for _ in 0..15 {
            tracker.update(-55);
        }
        assert_eq!(tracker.state, ProximityState::Near);
    }
}
