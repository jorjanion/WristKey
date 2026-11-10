//! Apple Continuity Protocol Parser for WristKey
//!
//! Decodes Apple-specific Bluetooth Low Energy Manufacturer Advertisements
//! (Manufacturer ID 0x004C), extracting Proximity Pairing, Nearby Info,
//! Handoff, and AirDrop state frames.
//!
//! Specifically validates Apple Watch on-wrist optical state and proximity.

pub const APPLE_MANUFACTURER_ID: u16 = 0x004C;
pub const APPLE_COMPANY_ID: u16 = 0x004C;

// Apple Continuity Message Types (byte 0 of payload)
pub const TYPE_AIRDROP: u8 = 0x05;
pub const TYPE_PROXIMITY_PAIRING: u8 = 0x07;
pub const TYPE_AIRPLAY_TARGET: u8 = 0x09;
pub const TYPE_MAGIC_SWITCH: u8 = 0x0B;
pub const TYPE_HANDOFF: u8 = 0x0C;
pub const TYPE_NEARBY_INFO: u8 = 0x10;
pub const TYPE_FIND_MY: u8 = 0x12;

// Nearby Info Action Codes (lower 4 bits of byte 2)
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleContinuityFrame {
    pub message_type: u8,
    pub message_type_name: String,
    pub action_code: Option<u8>,
    pub action_name: Option<String>,
    pub is_on_wrist: bool,
    pub is_locked: bool,
    pub is_on_wrist_and_unlocked: bool,
    pub auto_unlock_capable: bool,
    pub wifi_on: bool,
    pub is_primary_icloud: bool,
    pub airdrop_enabled: bool,
    pub auth_tag: Option<Vec<u8>>,
    pub raw_payload: Vec<u8>,
}

impl AppleContinuityFrame {
    /// Attempts to parse an Apple Continuity advertisement payload from raw manufacturer data.
    ///
    /// Accepts either the full manufacturer data slice (including 0x4C, 0x00 header) or
    /// just the Continuity payload bytes. Supports concatenated multi-TLV packets.
    pub fn parse(data: &[u8]) -> Option<Self> {
        let payload = if data.len() >= 2 && data[0] == 0x4C && data[1] == 0x00 {
            &data[2..]
        } else {
            data
        };

        if payload.len() < 2 {
            return None;
        }

        // Iterate through all TLV elements in the payload.
        // If multiple TLVs are present, prioritize Nearby Info (0x10) or Magic Switch (0x0B)
        // because those contain the Apple Watch wrist detection & auto-unlock state.
        let mut best_frame: Option<Self> = None;
        let mut offset = 0;

        while offset + 2 <= payload.len() {
            let msg_type = payload[offset];
            let msg_len = payload[offset + 1] as usize;
            let val_start = offset + 2;
            let val_end = val_start + msg_len;

            if val_end > payload.len() {
                // If standard TLV length exceeds remaining slice, parse remaining as single payload
                let sub = &payload[val_start..];
                if let Some(f) = Self::parse_tlv_payload(msg_type, sub, payload) {
                    if best_frame.is_none() || f.is_on_wrist_and_unlocked {
                        best_frame = Some(f);
                    }
                }
                break;
            }

            let tlv_slice = &payload[val_start..val_end];
            if let Some(f) = Self::parse_tlv_payload(msg_type, tlv_slice, payload) {
                // Prioritize frames that confirm an on-wrist unlocked Apple Watch
                if f.is_on_wrist_and_unlocked && f.is_apple_watch() {
                    return Some(f);
                }
                if best_frame.is_none() || (f.message_type == TYPE_NEARBY_INFO && best_frame.as_ref().map_or(true, |b| b.message_type != TYPE_NEARBY_INFO)) {
                    best_frame = Some(f);
                }
            }

            offset = val_end;
        }

        best_frame
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

                // Bit 0: Auth Tag length (0 = 3 bytes, 1 = 16 bytes auth tag)
                if (status_flags & 0x01) != 0 {
                    auth_tag_len = 16;
                }

                // Bit 2: Wi-Fi state
                wifi_on = (status_flags & 0x04) != 0;

                // Bit 5 (0x20): Display / Device Locked
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

            // Apple Watch Hardware Device Identification:
            // 1. Action Code 0x0A (ACTION_WATCH_ON_WRIST_UNLOCKED)
            // 2. Action Code 0x0B (ACTION_RECENT_INTERACTION - user touched screen, raised wrist, or interacted)
            // 3. Auto-Unlock capability bit in status flags (0xC0)
            // 4. Apple Hardware Device Class: When Nearby Info has length >= 7, payload[6] is (device_class << 3).
            //    Device Class 5 = Apple Watch (0x28)!
            let is_watch = action_code == ACTION_WATCH_ON_WRIST_UNLOCKED
                || action_code == ACTION_RECENT_INTERACTION
                || auto_unlock
                || (payload.len() >= 7 && (payload[payload.len() - 1] >> 3) == 5);

            // An Apple Watch is ONLY on wrist and unlocked if:
            // 1. Action Code is 0x0A (ACTION_WATCH_ON_WRIST_UNLOCKED), OR
            // 2. Action Code is 0x0B (ACTION_RECENT_INTERACTION) and device is NOT locked (!is_locked).
            // Under NO circumstances does the static auto_unlock capability flag (0xC0) imply on-wrist!
            // When resting on the charging dock or on a table (Action 0x05, 0x03, 0x07, 0x00), it is NOT on wrist!
            let is_on_wrist = is_watch
                && (action_code == ACTION_WATCH_ON_WRIST_UNLOCKED || (action_code == ACTION_RECENT_INTERACTION && !is_locked))
                && !is_locked;
            let is_on_wrist_and_unlocked = is_on_wrist;

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
        } else if message_type == TYPE_MAGIC_SWITCH {
            // Magic Switch (0x0B) is Apple Watch-specific.
            // Payload length is typically 3 bytes: 2 bytes of data + 1 byte of confidence:
            // 0x03 = Not on wrist (removed / off wrist / charging dock)
            // 0x1F = Wrist detection disabled
            // 0x3F = On wrist
            let is_on_wrist = if !payload.is_empty() {
                let conf = payload[payload.len() - 1];
                conf == 0x3F || (conf != 0x03 && conf != 0x1F)
            } else {
                false
            };

            Some(Self {
                message_type,
                message_type_name: type_name,
                action_code: None,
                action_name: None,
                is_on_wrist,
                is_locked: false,
                is_on_wrist_and_unlocked: is_on_wrist,
                auto_unlock_capable: true,
                wifi_on: false,
                is_primary_icloud: false,
                airdrop_enabled: false,
                auth_tag: None,
                raw_payload: raw.to_vec(),
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
                raw_payload: raw.to_vec(),
            })
        }
    }

    /// Returns true if this frame is confirmed to originate from an Apple Watch:
    /// - Magic Switch frame (0x0B) - Apple Watch wrist beacon
    /// - Action Code 0x0A (Watch on wrist & unlocked)
    /// - Action Code 0x0B (Recent user interaction / screen touch)
    /// - Auto Unlock capability flags enabled (Bit 6 or 7 of status byte)
    /// - Hardware Device Class 5 == Apple Watch (0x28 byte in Nearby Info payload >= 7 bytes)
    pub fn is_confirmed_apple_watch(&self) -> bool {
        match self.message_type {
            TYPE_MAGIC_SWITCH => true,
            TYPE_NEARBY_INFO => {
                if self.action_code == Some(ACTION_WATCH_ON_WRIST_UNLOCKED)
                    || self.action_code == Some(ACTION_RECENT_INTERACTION)
                    || self.auto_unlock_capable
                {
                    return true;
                }
                // Hardware Device Class 5 == Apple Watch:
                // In Apple Continuity Nearby Info (0x10), when payload length is >= 7 bytes,
                // the device class byte is payload[payload.len() - 1].
                // (device_class_byte >> 3) == 5 represents an Apple Watch (e.g. 0x28, 0x29).
                if self.raw_payload.len() >= 9 && self.raw_payload[0] == TYPE_NEARBY_INFO {
                    let len = self.raw_payload[1] as usize;
                    if len >= 7 && self.raw_payload.len() >= 2 + len {
                        let dev_class_byte = self.raw_payload[2 + len - 1];
                        if (dev_class_byte >> 3) == 5 {
                            return true;
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Returns true if this frame is confirmed to originate from an Apple Watch:
    /// - Magic Switch frame (0x0B)
    /// - Nearby Info (0x10) with On-Wrist action code (0x0A) or Auto-Unlock capability flags (0xC0)
    /// Explicitly excludes iPhones, iPads, Macs, and AirPods.
    pub fn is_apple_watch(&self) -> bool {
        self.is_confirmed_apple_watch()
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
        // Apple Watch screen active with AutoUnlock capable flag set (0x44):
        // Device is an Apple Watch, but screen-on by itself (e.g. nightstand/charger mode) is NOT on-wrist!
        let raw_screen_on = vec![0x10, 0x05, 0x17, 0x44, 0x11, 0x22, 0x33];
        let frame = AppleContinuityFrame::parse(&raw_screen_on).expect("Must parse");
        assert_eq!(frame.action_code, Some(ACTION_SCREEN_ON));
        assert!(frame.is_primary_icloud);
        assert!(!frame.is_locked);
        assert!(!frame.is_on_wrist);
        assert!(!frame.is_on_wrist_and_unlocked);
        assert!(frame.is_apple_watch());

        // Action 0x1B (Recent Interaction with AutoUnlock capable flag set)
        let raw_interaction = vec![0x10, 0x05, 0x1B, 0x44, 0x11, 0x22, 0x33];
        let frame2 = AppleContinuityFrame::parse(&raw_interaction).expect("Must parse");
        assert_eq!(frame2.action_code, Some(ACTION_RECENT_INTERACTION));
        assert!(!frame2.is_locked);
        assert!(frame2.is_on_wrist);
        assert!(frame2.is_on_wrist_and_unlocked);
        assert!(frame2.is_apple_watch());
    }

    #[test]
    fn test_parse_nearby_info_interaction_without_autounlock_flag() {
        // Recent interaction (action 0x0B) with status 0x04 (Wi-Fi on, no auto-unlock flag)
        let raw = vec![0x10, 0x05, 0x1B, 0x04, 0x11, 0x22, 0x33];
        let frame = AppleContinuityFrame::parse(&raw).expect("Must parse");
        assert_eq!(frame.action_code, Some(ACTION_RECENT_INTERACTION));
        assert!(!frame.is_locked);
        assert!(frame.is_on_wrist);
        assert!(frame.is_on_wrist_and_unlocked);
        assert!(frame.is_apple_watch());
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
        assert!(!frame_handoff.is_apple_watch());

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
        // Magic Switch (0x0B) with Confidence 0x3F (On wrist)
        let raw_magic = vec![0x0B, 0x03, 0xC4, 0xB5, 0x3F];
        let frame = AppleContinuityFrame::parse(&raw_magic).expect("Must parse");
        assert_eq!(frame.message_type, TYPE_MAGIC_SWITCH);
        assert!(frame.is_apple_watch());
        assert!(frame.is_on_wrist);
        assert!(frame.is_on_wrist_and_unlocked);

        // Magic Switch (0x0B) with Confidence 0x03 (Off wrist / Charging)
        let raw_magic_off = vec![0x0B, 0x03, 0xC4, 0xB5, 0x03];
        let frame_off = AppleContinuityFrame::parse(&raw_magic_off).expect("Must parse");
        assert_eq!(frame_off.message_type, TYPE_MAGIC_SWITCH);
        assert!(frame_off.is_apple_watch());
        assert!(!frame_off.is_on_wrist);
        assert!(!frame_off.is_on_wrist_and_unlocked);
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

    #[test]
    fn test_parse_apple_watch_se_live_telemetry() {
        // Captured live from user's Apple Watch SE (watchOS 10.6.2) on wrist:
        // Type 0x10 (Nearby Info), Length 0x07, Action 0x3B (0x0B Recent Interaction), Status 0x1F, Auth Tag/Entropy, Device Class 0x28 (0x28 >> 3 == 5: Apple Watch)
        let raw = vec![0x10, 0x07, 0x3B, 0x1F, 0x2E, 0xD6, 0xDB, 0x68, 0x28];
        let frame = AppleContinuityFrame::parse(&raw).expect("Must parse Apple Watch SE live frame");

        assert_eq!(frame.message_type, TYPE_NEARBY_INFO);
        assert!(frame.is_confirmed_apple_watch());
        assert!(frame.is_apple_watch());
        assert!(frame.is_on_wrist);
        assert!(frame.is_on_wrist_and_unlocked);
        assert!(!frame.is_locked);
    }

    #[test]
    fn test_parse_apple_watch_on_charger_not_on_wrist() {
        // Captured live from user's Apple Watch SE on charging dock:
        // Action 0x35 (0x05 Audio/Charging state), Status 0x1F, Device Class 0x28 (Apple Watch)
        let raw = vec![0x10, 0x07, 0x35, 0x1F, 0x1F, 0x66, 0xAD, 0x73, 0x28];
        let frame = AppleContinuityFrame::parse(&raw).expect("Must parse Watch on charger frame");

        assert_eq!(frame.message_type, TYPE_NEARBY_INFO);
        assert!(frame.is_confirmed_apple_watch());
        assert!(frame.is_apple_watch());
        // Must NOT be considered on-wrist while on the charger!
        assert!(!frame.is_on_wrist);
        assert!(!frame.is_on_wrist_and_unlocked);
    }
}
