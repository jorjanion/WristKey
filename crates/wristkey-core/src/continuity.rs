use serde::{Deserialize, Serialize};

/// Apple's Bluetooth SIG assigned company identifier (16-bit).
pub const APPLE_COMPANY_ID: u16 = 0x004C;

/// Apple Continuity message types.
pub const TYPE_AIRDROP: u8 = 0x05;
pub const TYPE_PROXIMITY_PAIRING: u8 = 0x07;
pub const TYPE_HANDOFF: u8 = 0x0C;
pub const TYPE_NEARBY_INFO: u8 = 0x10;
pub const TYPE_FIND_MY: u8 = 0x12;

/// Action codes inside Nearby Info (0x10) messages (lower 4 bits of payload[0]).
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

/// Decoded representation of an Apple Continuity advertisement payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppleContinuityFrame {
    pub message_type: u8,
    pub message_type_name: String,
    pub action_code: Option<u8>,
    pub action_name: Option<String>,
    pub is_on_wrist_and_unlocked: bool,
    pub auto_unlock_capable: bool,
    pub raw_payload: Vec<u8>,
}

impl AppleContinuityFrame {
    /// Attempts to parse an Apple manufacturer data payload.
    ///
    /// Expected format in Apple BLE advertising:
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
            // Byte 1: Length of remainder (e.g. 0x05)
            // Byte 2: [Status Flags (upper 4 bits) | Action Code (lower 4 bits)]
            let action_byte = data[2];
            let action_code = action_byte & 0x0F;

            let action_name = match action_code {
                ACTION_UNKNOWN => "Unknown",
                ACTION_ACTIVITY_DISABLED => "Activity Reporting Disabled",
                ACTION_IDLE => "Idle User",
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

            let is_on_wrist = action_code == ACTION_WATCH_ON_WRIST_UNLOCKED;

            // Byte 3 (if present): Status bits
            let mut auto_unlock = false;
            if data.len() >= 4 {
                let status_flags = data[3];
                // Bit 6 (0x40) and Bit 7 (0x80) indicate Auto Unlock readiness
                auto_unlock = (status_flags & 0xC0) != 0;
            }

            Some(Self {
                message_type,
                message_type_name: type_name,
                action_code: Some(action_code),
                action_name: Some(action_name),
                is_on_wrist_and_unlocked: is_on_wrist,
                auto_unlock_capable: auto_unlock,
                raw_payload: data.to_vec(),
            })
        } else {
            Some(Self {
                message_type,
                message_type_name: type_name,
                action_code: None,
                action_name: None,
                is_on_wrist_and_unlocked: false,
                auto_unlock_capable: false,
                raw_payload: data.to_vec(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nearby_info_watch_on_wrist() {
        // [0x10 (Nearby Info), 0x05 (len), 0x0A (Action: Watch on wrist), 0x44 (Status flags with auto-unlock), 0x11, 0x22, 0x33]
        let raw = vec![0x10, 0x05, 0x0A, 0x44, 0x11, 0x22, 0x33];
        let frame = AppleContinuityFrame::parse(&raw).expect("Must parse");

        assert_eq!(frame.message_type, TYPE_NEARBY_INFO);
        assert_eq!(frame.action_code, Some(ACTION_WATCH_ON_WRIST_UNLOCKED));
        assert!(frame.is_on_wrist_and_unlocked);
        assert!(frame.auto_unlock_capable);
        assert_eq!(frame.action_name.unwrap(), "Watch On Wrist & Unlocked");
    }

    #[test]
    fn test_parse_nearby_info_idle_screen() {
        let raw = vec![0x10, 0x05, 0x03, 0x00, 0x00, 0x00, 0x00];
        let frame = AppleContinuityFrame::parse(&raw).expect("Must parse");

        assert_eq!(frame.action_code, Some(ACTION_IDLE));
        assert!(!frame.is_on_wrist_and_unlocked);
    }
}
