use serde::{Deserialize, Serialize};

/// Apple's Bluetooth SIG assigned company identifier (16-bit).
pub const APPLE_COMPANY_ID: u16 = 0x004C;

/// Apple Continuity message types.
pub const TYPE_AIRDROP: u8 = 0x05;
pub const TYPE_PROXIMITY_PAIRING: u8 = 0x07;
pub const TYPE_MAGIC_SWITCH: u8 = 0x0B;
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
    /// Can contain a single TLV or multiple concatenated TLVs.
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }

        let mut frames = Vec::new();
        let mut offset = 0;

        while offset + 2 <= data.len() {
            let msg_type = data[offset];
            let msg_len = data[offset + 1] as usize;
            let end = offset + 2 + msg_len;
            if end > data.len() {
                if let Some(frame) = Self::parse_tlv_payload(msg_type, &data[offset + 2..], &data[offset..]) {
                    frames.push(frame);
                }
                break;
            }

            let tlv_data = &data[offset + 2..end];
            let raw_tlv = &data[offset..end];
            if let Some(frame) = Self::parse_tlv_payload(msg_type, tlv_data, raw_tlv) {
                frames.push(frame);
            }

            offset = end;
        }

        if frames.is_empty() {
            let msg_type = data[0];
            let tlv_data = if data.len() > 2 { &data[2..] } else { &[] };
            return Self::parse_tlv_payload(msg_type, tlv_data, data);
        }

        // Prioritize frames:
        // 0. Confirmed Apple Watch on wrist & unlocked
        // 1. Any Apple Watch candidate frame
        // 2. Other Nearby Info frames
        // 3. Handoff or Proximity Pairing frames
        // 4. Any other frame
        frames.sort_by_key(|f| {
            if f.is_on_wrist_and_unlocked && f.is_apple_watch() {
                0
            } else if f.is_apple_watch() {
                1
            } else if f.message_type == TYPE_NEARBY_INFO {
                2
            } else if f.message_type == TYPE_HANDOFF || f.message_type == TYPE_PROXIMITY_PAIRING {
                3
            } else {
                4
            }
        });

        frames.into_iter().next()
    }

    fn parse_tlv_payload(message_type: u8, payload: &[u8], raw: &[u8]) -> Option<Self> {
        let type_name = match message_type {
            TYPE_AIRDROP => "AirDrop",
            TYPE_PROXIMITY_PAIRING => "Proximity Pairing",
            TYPE_MAGIC_SWITCH => "Magic Switch",
            TYPE_HANDOFF => "Handoff",
            TYPE_NEARBY_INFO => "Nearby Info",
            TYPE_FIND_MY => "Find My",
            0x01 => "iBeacon",
            0x06 => "HomeKit",
            0x08 => "Siri",
            0x0E => "Instant Hotspot",
            0x0F => "Wi-Fi Password",
            _ => "Apple Continuity (Other)",
        }
        .to_string();

        if message_type == TYPE_NEARBY_INFO && !payload.is_empty() {
            // payload[0] corresponds to byte 2 of the wire packet:
            // [Status Flags (upper 4 bits) | Action Code (lower 4 bits)]
            let byte2 = payload[0];
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

            if payload.len() >= 2 {
                // payload[1] corresponds to byte 3: System Status flags (8 bits)
                let status_flags = payload[1];

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

            // Extract authentication tag if present (payload[2..])
            let auth_tag = if payload.len() >= 2 + auth_tag_len {
                Some(payload[2..2 + auth_tag_len].to_vec())
            } else if payload.len() > 2 {
                Some(payload[2..].to_vec())
            } else {
                None
            };

            // Apple Watch Wrist Detection & Lock status:
            // On Apple Watch with Wrist Detection enabled (default on watchOS):
            // - Bit 5 (0x20) of status byte: 0 = Unlocked (worn on wrist), 1 = Locked (off wrist or locked)
            // - Action code 0x0A: Explicitly "Watch on wrist & unlocked"
            // - If the watch is locked (is_locked == true), it is off wrist or locked with passcode.
            // - If not locked (!is_locked), it is on wrist and unlocked.
            let is_on_wrist = if action_code == ACTION_WATCH_ON_WRIST_UNLOCKED {
                true
            } else if is_locked {
                false
            } else {
                // Not locked: device is worn on wrist and actively operating
                true
            };

            let is_on_wrist_and_unlocked = is_on_wrist && !is_locked;

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
                raw_payload: raw.to_vec(),
            })
        } else {
            let is_magic_switch = message_type == TYPE_MAGIC_SWITCH;
            Some(Self {
                message_type,
                message_type_name: type_name,
                action_code: None,
                action_name: None,
                is_on_wrist: is_magic_switch,
                is_locked: false,
                is_on_wrist_and_unlocked: is_magic_switch,
                auto_unlock_capable: is_magic_switch,
                wifi_on: false,
                is_primary_icloud: false,
                airdrop_enabled: false,
                auth_tag: None,
                raw_payload: raw.to_vec(),
            })
        }
    }

    /// Returns true if this frame is confirmed to originate from an Apple Watch:
    /// - Magic Switch frame (0x0B) - Apple Watch wrist beacon
    /// - Action Code 0x0A (Watch on wrist & unlocked)
    /// - Auto Unlock capability flags enabled (Bit 6 or 7 of status byte)
    pub fn is_confirmed_apple_watch(&self) -> bool {
        match self.message_type {
            TYPE_MAGIC_SWITCH => true,
            TYPE_NEARBY_INFO => {
                self.action_code == Some(ACTION_WATCH_ON_WRIST_UNLOCKED)
                    || self.auto_unlock_capable
            }
            _ => false,
        }
    }

    /// Returns true if this frame can potentially be an Apple Watch:
    /// - Any confirmed Apple Watch
    /// - Nearby Info (0x10) with watch-compatible states (Screen On 0x07, Interaction 0x0B, Idle 0x03, 0x0A, etc.)
    /// - Handoff (0x0C)
    /// Explicitly excludes AirTags/Find My trackers (0x12, 0x16), AirPods (0x07 pairing), and iPhone-exclusive call/driving/video states.
    pub fn is_apple_watch(&self) -> bool {
        if self.is_confirmed_apple_watch() {
            return true;
        }

        match self.message_type {
            TYPE_NEARBY_INFO => {
                if let Some(action) = self.action_code {
                    // Exclude non-watch iPhone-exclusive activities:
                    if action == ACTION_SCREEN_ON_VIDEO
                        || action == ACTION_DRIVING
                        || action == ACTION_CALL_ACTIVE
                        || action == ACTION_AUDIO_PLAYING_LOCKED
                    {
                        return false;
                    }
                }
                true
            }
            TYPE_HANDOFF => true,
            _ => false,
        }
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
        assert!(frame.is_apple_watch());
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
        // It has auto_unlock_capable bit set, so it's recognized as an Apple Watch (locked)
        assert!(frame.is_apple_watch());
    }

    #[test]
    fn test_parse_nearby_info_watch_active_screen_on_and_interaction() {
        // Apple Watch SE screen active with iCloud account signed in (0x17 = 0x10 iCloud + 0x07 Screen On)
        // unlocked (0x04 Wi-Fi, Lock bit 5 is 0):
        let raw_screen_on = vec![0x10, 0x05, 0x17, 0x04, 0x11, 0x22, 0x33];
        let frame = AppleContinuityFrame::parse(&raw_screen_on).expect("Must parse");
        assert_eq!(frame.action_code, Some(ACTION_SCREEN_ON));
        assert!(frame.is_primary_icloud);
        assert!(!frame.is_locked);
        assert!(frame.is_on_wrist);
        assert!(frame.is_on_wrist_and_unlocked);
        assert!(frame.is_apple_watch());

        // Action 0x1B (Recent Interaction with iCloud signed in)
        let raw_interaction = vec![0x10, 0x05, 0x1B, 0x04, 0x11, 0x22, 0x33];
        let frame2 = AppleContinuityFrame::parse(&raw_interaction).expect("Must parse");
        assert_eq!(frame2.action_code, Some(ACTION_RECENT_INTERACTION));
        assert!(!frame2.is_locked);
        assert!(frame2.is_on_wrist);
        assert!(frame2.is_on_wrist_and_unlocked);
        assert!(frame2.is_apple_watch());
    }

    #[test]
    fn test_parse_nearby_info_iphone_rejected() {
        // iPhone playing video / screen active (action code 0x09)
        let raw_video = vec![0x10, 0x05, 0x19, 0x04, 0x11, 0x22, 0x33];
        let frame_video = AppleContinuityFrame::parse(&raw_video).expect("Must parse");
        assert_eq!(frame_video.action_code, Some(ACTION_SCREEN_ON_VIDEO));
        assert!(frame_video.is_primary_icloud);
        assert!(!frame_video.is_apple_watch());
        assert!(!frame_video.is_confirmed_apple_watch());

        // iPhone phone call active (action code 0x0E)
        let raw_call = vec![0x10, 0x05, 0x1E, 0x04, 0x11, 0x22, 0x33];
        let frame_call = AppleContinuityFrame::parse(&raw_call).expect("Must parse");
        assert_eq!(frame_call.action_code, Some(ACTION_CALL_ACTIVE));
        assert!(frame_call.is_primary_icloud);
        assert!(!frame_call.is_apple_watch());
        assert!(!frame_call.is_confirmed_apple_watch());

        // iPhone driving (action code 0x0D)
        let raw_driving = vec![0x10, 0x05, 0x1D, 0x04, 0x11, 0x22, 0x33];
        let frame_driving = AppleContinuityFrame::parse(&raw_driving).expect("Must parse");
        assert_eq!(frame_driving.action_code, Some(ACTION_DRIVING));
        assert!(!frame_driving.is_apple_watch());
    }

    #[test]
    fn test_parse_handoff_and_airpods_proximity_pairing() {
        // Standalone Handoff (0x0C)
        let raw_handoff = vec![0x0C, 0x03, 0x01, 0x02, 0x03];
        let frame_handoff = AppleContinuityFrame::parse(&raw_handoff).expect("Must parse");
        assert_eq!(frame_handoff.message_type, TYPE_HANDOFF);
        assert!(!frame_handoff.is_on_wrist);
        assert!(!frame_handoff.is_confirmed_apple_watch());
        // Candidate frame
        assert!(frame_handoff.is_apple_watch());

        // Proximity Pairing (0x07) from AirPods
        let raw_pairing = vec![0x07, 0x02, 0x01, 0x02];
        let frame_pairing = AppleContinuityFrame::parse(&raw_pairing).expect("Must parse");
        assert_eq!(frame_pairing.message_type, TYPE_PROXIMITY_PAIRING);
        assert!(!frame_pairing.is_on_wrist);
        assert!(!frame_pairing.is_apple_watch());
        assert!(!frame_pairing.is_confirmed_apple_watch());
    }

    #[test]
    fn test_parse_magic_switch_watch() {
        // Magic Switch (0x0B) is Apple Watch-specific
        let raw_magic = vec![0x0B, 0x03, 0xC4, 0xB5, 0x3F];
        let frame = AppleContinuityFrame::parse(&raw_magic).expect("Must parse");
        assert_eq!(frame.message_type, TYPE_MAGIC_SWITCH);
        assert!(frame.is_apple_watch());
        assert!(frame.is_on_wrist);
        assert!(frame.is_on_wrist_and_unlocked);
    }

    #[test]
    fn test_parse_multi_tlv_concatenated_payload() {
        // Multi-TLV payload: Proximity Pairing (0x07, len 2) followed by Nearby Info (0x10, len 5)
        let raw = vec![
            0x07, 0x02, 0xAA, 0xBB,
            0x10, 0x05, 0x1A, 0x44, 0x11, 0x22, 0x33,
        ];
        let frame = AppleContinuityFrame::parse(&raw).expect("Must parse multi-TLV");
        // Nearby Info with on-wrist has higher priority
        assert_eq!(frame.message_type, TYPE_NEARBY_INFO);
        assert_eq!(frame.action_code, Some(ACTION_WATCH_ON_WRIST_UNLOCKED));
        assert!(frame.is_on_wrist_and_unlocked);
        assert!(frame.is_apple_watch());
    }
}
