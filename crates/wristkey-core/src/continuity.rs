use serde::{Deserialize, Serialize};

/// Apple's Bluetooth SIG assigned company identifier (16-bit).
pub const APPLE_COMPANY_ID: u16 = 0x004C;

/// Apple Continuity message types.
pub const TYPE_AIRDROP: u8 = 0x05;
pub const TYPE_PROXIMITY_PAIRING: u8 = 0x07;
pub const TYPE_HANDOFF: u8 = 0x0C;
pub const TYPE_NEARBY_INFO: u8 = 0x10;
pub const TYPE_FIND_MY: u8 = 0x12;

/// Action codes inside Nearby Info (0x10) messages (lower 4 bits of payload[2]).
pub const ACTION_UNKNOWN: u8 = 0x00;
pub const ACTION_ACTIVITY_DISABLED: u8 = 0x01;
pub const ACTION_IDLE: u8 = 0x03;
pub const ACTION_AUDIO_PLAYING_LOCKED: u8 = 0x05;
pub const ACTION_SCREEN_ON: u8 = 0x07;
pub const ACTION_SCREEN_ON_VIDEO: u8 = 0x09;
pub const ACTION_WATCH_ON_WRIST_UNLOCKED: u8 = 0x0A;
pub const ACTION_RECENT_INTERACTION: u8 = 0x0B;
pub const ACTION_DRIVING: u8 = 0x0D;
pub const ACTION_CALL_ACTIVE: u8 = 0x0E;

/// Detailed decoded representation of an Apple Continuity advertisement payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppleContinuityFrame {
    /// Protocol type code (e.g. 0x10 for Nearby Info).
    pub message_type: u8,
    /// Human-readable message type name.
    pub message_type_name: String,
    /// Action code from Nearby Info (lower 4 bits of byte 2).
    pub action_code: Option<u8>,
    /// Human-readable action description.
    pub action_name: Option<String>,
    /// True if the watch is detected on the user's wrist.
    pub is_on_wrist: bool,
    /// True if the watch is currently locked with a passcode.
    pub is_locked: bool,
    /// Strict combined status: On-Wrist AND Passcode Unlocked.
    pub is_on_wrist_and_unlocked: bool,
    /// Apple Watch Auto Unlock capability enabled (Bit 6/7).
    pub auto_unlock_capable: bool,
    /// Device Wi-Fi interface status (Bit 2 of status byte).
    pub wifi_on: bool,
    /// Device is marked as the primary iCloud account device (Bit 4 of byte 2).
    pub is_primary_icloud: bool,
    /// AirDrop receiving enabled (Bit 5 of byte 2).
    pub airdrop_enabled: bool,
    /// Message integrity authentication tag if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_tag: Option<Vec<u8>>,
    /// Raw manufacturer payload bytes.
    pub raw_payload: Vec<u8>,
}

impl AppleContinuityFrame {
    /// Parses an Apple manufacturer data payload.
    ///
    /// Wire format:
    /// `[Type (1 byte)] [Length (1 byte)] [Data (Length bytes)...]`
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }

        let message_type = data[0];
        let type_name = match message_type {
            TYPE_AIRDROP => "AirDrop",
            TYPE_PROXIMITY_PAIRING => "Proximity Pairing",
            TYPE_HANDOFF => "Handoff",
            TYPE_NEARBY_INFO => "Nearby Info",
            TYPE_FIND_MY => "Find My",
            0x01 => "iBeacon",
            0x06 => "HomeKit",
            0x08 => "Siri",
            0x0B => "Trackpad",
            0x0E => "Instant Hotspot",
            0x0F => "Wi-Fi Password",
            _ => "Apple Continuity (Other)",
        }
        .to_string();

        if message_type == TYPE_NEARBY_INFO && data.len() >= 3 {
            // Byte 0: Type (0x10)
            // Byte 1: Length of payload (e.g. 0x05)
            // Byte 2: [Status Flags (upper 4 bits) | Action Code (lower 4 bits)]
            let byte2 = data[2];
            let is_primary_icloud = (byte2 & 0x10) != 0;
            let airdrop_enabled = (byte2 & 0x20) != 0;
            let action_code = byte2 & 0x0F;

            let action_name = match action_code {
                ACTION_UNKNOWN => "Unknown",
                ACTION_ACTIVITY_DISABLED => "Activity Reporting Disabled",
                ACTION_IDLE => "Idle User (Screen Off)",
                ACTION_AUDIO_PLAYING_LOCKED => "Audio Playing (Screen Locked)",
                ACTION_SCREEN_ON => "Active User (Screen On)",
                ACTION_SCREEN_ON_VIDEO => "Video Playing (Screen On)",
                ACTION_WATCH_ON_WRIST_UNLOCKED => "Watch On Wrist & Unlocked",
                ACTION_RECENT_INTERACTION => "Recent User Interaction",
                ACTION_DRIVING => "Driving Vehicle",
                ACTION_CALL_ACTIVE => "Phone / FaceTime Call",
                _ => "Other Action",
            }
            .to_string();

            // Default flags if status byte not present
            let mut is_locked = false;
            let mut auto_unlock = false;
            let mut wifi_on = false;
            let mut auth_tag_len = 3usize;

            if data.len() >= 4 {
                // Byte 3: System Status flags (8 bits)
                let status_flags = data[3];

                // Bit 1 (0x02): Auth tag length (0 = 3 bytes, 1 = 4 bytes)
                if (status_flags & 0x02) != 0 {
                    auth_tag_len = 4;
                }

                // Bit 2 (0x04): Wi-Fi state
                wifi_on = (status_flags & 0x04) != 0;

                // Bit 5 (0x20): Apple Watch Lock Status: 1 = Locked, 0 = Unlocked
                is_locked = (status_flags & 0x20) != 0;

                // Bit 6 (0x40) and Bit 7 (0x80): Auto Unlock capability
                auto_unlock = (status_flags & 0xC0) != 0;
            }

            // Extract authentication tag if present
            let auth_tag = if data.len() >= 4 + auth_tag_len {
                Some(data[4..4 + auth_tag_len].to_vec())
            } else if data.len() > 4 {
                Some(data[4..].to_vec())
            } else {
                None
            };

            // An Apple Watch is on-wrist strictly when action code is 0x0A (Watch on wrist & unlocked).
            // iPhones, iPads, and Macs NEVER emit action code 0x0A.
            let is_on_wrist = action_code == ACTION_WATCH_ON_WRIST_UNLOCKED;
            let is_on_wrist_and_unlocked = action_code == ACTION_WATCH_ON_WRIST_UNLOCKED && !is_locked;

            Some(Self {
                message_type,
                message_type_name: type_name,
                action_code: Some(action_code),
                action_name: Some(action_name),
                is_on_wrist,
                is_locked,
                is_on_wrist_and_unlocked,
                auto_unlock_capable: auto_unlock,
                wifi_on,
                is_primary_icloud,
                airdrop_enabled,
                auth_tag,
                raw_payload: data.to_vec(),
            })
        } else {
            Some(Self {
                message_type,
                message_type_name: type_name,
                action_code: None,
                action_name: None,
                is_on_wrist: false,
                is_locked: false,
                is_on_wrist_and_unlocked: false,
                auto_unlock_capable: false,
                wifi_on: false,
                is_primary_icloud: false,
                airdrop_enabled: false,
                auth_tag: None,
                raw_payload: data.to_vec(),
            })
        }
    }

    /// Returns true if this frame exhibits Apple Watch specific characteristics
    /// (strictly Action Code 0x0A: Watch on wrist & unlocked).
    pub fn is_apple_watch(&self) -> bool {
        self.message_type == TYPE_NEARBY_INFO
            && self.action_code == Some(ACTION_WATCH_ON_WRIST_UNLOCKED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nearby_info_watch_on_wrist_and_unlocked() {
        // [0x10 (Nearby Info), 0x05 (len), 0x1A (iCloud primary + Action 0x0A: On wrist), 0x44 (Auto-unlock=1, Wi-Fi=1, Lock=0), 0x11, 0x22, 0x33]
        let raw = vec![0x10, 0x05, 0x1A, 0x44, 0x11, 0x22, 0x33];
        let frame = AppleContinuityFrame::parse(&raw).expect("Must parse");

        assert_eq!(frame.message_type, TYPE_NEARBY_INFO);
        assert_eq!(frame.action_code, Some(ACTION_WATCH_ON_WRIST_UNLOCKED));
        assert!(frame.is_primary_icloud);
        assert!(frame.wifi_on);
        assert!(!frame.is_locked);
        assert!(frame.is_on_wrist);
        assert!(frame.is_on_wrist_and_unlocked);
        assert!(frame.auto_unlock_capable);
        assert_eq!(frame.auth_tag, Some(vec![0x11, 0x22, 0x33]));
    }

    #[test]
    fn test_parse_nearby_info_watch_removed_from_wrist_locked() {
        // Watch removed: Lock bit 5 (0x20) is set, action is idle (0x03)
        // Status = 0x64 (AutoUnlock=0x40 | Locked=0x20 | WiFi=0x04)
        let raw = vec![0x10, 0x05, 0x03, 0x64, 0xAA, 0xBB, 0xCC];
        let frame = AppleContinuityFrame::parse(&raw).expect("Must parse");

        assert_eq!(frame.action_code, Some(ACTION_IDLE));
        assert!(frame.is_locked);
        assert!(!frame.is_on_wrist_and_unlocked);
        assert!(!frame.is_on_wrist);
        assert!(!frame.is_apple_watch());
    }

    #[test]
    fn test_parse_nearby_info_iphone_rejected() {
        // iPhone screen active on desk: action code 0x07 (Screen On), unlocked (status 0x44)
        let raw = vec![0x10, 0x05, 0x07, 0x44, 0x11, 0x22, 0x33];
        let frame = AppleContinuityFrame::parse(&raw).expect("Must parse");

        assert_eq!(frame.action_code, Some(ACTION_SCREEN_ON));
        assert!(!frame.is_locked);
        assert!(!frame.is_on_wrist);
        assert!(!frame.is_on_wrist_and_unlocked);
        assert!(!frame.is_apple_watch());
    }
}
